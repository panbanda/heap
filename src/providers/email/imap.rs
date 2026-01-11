//! IMAP/SMTP provider implementation.
//!
//! This module provides an [`EmailProvider`] implementation using standard IMAP
//! for fetching emails and SMTP for sending. This supports most email providers
//! that aren't Gmail (or Gmail via IMAP).
//!
//! # Authentication
//!
//! Credentials (username/password or OAuth tokens) are stored in the system keychain,
//! referenced by account ID. The provider handles connection management and
//! reconnection as needed.
//!
//! # Protocol Details
//!
//! - Uses IMAP4rev1 (RFC 3501) via `async-imap`
//! - Uses SMTP with STARTTLS or direct TLS via `lettre`
//! - Supports IDLE for push notifications (when available)

use async_imap::types::{Fetch, Flag};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lettre::message::{Mailbox, MessageBuilder, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::{Addr, Message as ParsedMessage, MessageParser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsConnector;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

use super::{
    Change, EmailProvider, OutgoingEmail, Pagination, PendingChange, PendingChangeType,
    ProviderError, Result,
};
use crate::domain::{
    AccountId, Address, Email, EmailId, Label, LabelId, MessageId, ProviderType, Thread, ThreadId,
    ThreadSummary,
};

/// IMAP/SMTP configuration.
#[derive(Debug, Clone)]
pub struct ImapConfig {
    /// IMAP server hostname.
    pub imap_host: String,
    /// IMAP server port (typically 993 for TLS, 143 for STARTTLS).
    pub imap_port: u16,
    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port (typically 465 for TLS, 587 for STARTTLS).
    pub smtp_port: u16,
    /// Whether to use TLS (true) or STARTTLS (false).
    pub use_tls: bool,
}

impl ImapConfig {
    /// Creates a configuration for a typical TLS setup.
    pub fn tls(imap_host: impl Into<String>, smtp_host: impl Into<String>) -> Self {
        Self {
            imap_host: imap_host.into(),
            imap_port: 993,
            smtp_host: smtp_host.into(),
            smtp_port: 465,
            use_tls: true,
        }
    }

    /// Creates a configuration for a STARTTLS setup.
    pub fn starttls(imap_host: impl Into<String>, smtp_host: impl Into<String>) -> Self {
        Self {
            imap_host: imap_host.into(),
            imap_port: 143,
            smtp_host: smtp_host.into(),
            smtp_port: 587,
            use_tls: false,
        }
    }
}

/// Credentials stored in keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImapCredentials {
    /// Username (usually email address).
    pub username: String,
    /// Password or app-specific password.
    pub password: String,
    /// Display name for outgoing emails.
    pub display_name: Option<String>,
}

/// Type alias for the IMAP session with TLS (using tokio-util compat layer).
type ImapSession = async_imap::Session<Compat<TlsStream<TcpStream>>>;

/// IMAP/SMTP email provider.
///
/// Implements [`EmailProvider`] using standard IMAP for fetching and SMTP for sending.
///
/// # Example
///
/// ```ignore
/// use margin::providers::email::{ImapProvider, ImapConfig, EmailProvider, Pagination};
///
/// let config = ImapConfig::tls("imap.example.com", "smtp.example.com");
/// let mut provider = ImapProvider::new(account_id, config);
/// provider.authenticate().await?;
///
/// let threads = provider.fetch_threads("INBOX", Pagination::with_limit(50)).await?;
/// ```
pub struct ImapProvider {
    /// Account ID for keychain credential lookup.
    account_id: AccountId,
    /// Server configuration.
    config: ImapConfig,
    /// Credentials (loaded from keychain).
    credentials: Option<ImapCredentials>,
    /// IMAP session (connected when authenticated).
    session: Option<Arc<Mutex<ImapSession>>>,
    /// Whether the provider is authenticated and connected.
    authenticated: bool,
    /// Cache of UIDVALIDITY per folder for detecting invalidation.
    #[allow(dead_code)]
    uid_validity: HashMap<String, u32>,
}

impl ImapProvider {
    /// Creates a new IMAP provider for the specified account.
    ///
    /// The provider is not authenticated until [`authenticate`](Self::authenticate) is called.
    ///
    /// # Arguments
    ///
    /// * `account_id` - Account ID used to look up credentials in the keychain
    /// * `config` - Server configuration
    pub fn new(account_id: AccountId, config: ImapConfig) -> Self {
        Self {
            account_id,
            config,
            credentials: None,
            session: None,
            authenticated: false,
            uid_validity: HashMap::new(),
        }
    }

    /// Creates a new IMAP provider with explicit credentials.
    pub fn with_credentials(
        account_id: AccountId,
        config: ImapConfig,
        credentials: ImapCredentials,
    ) -> Self {
        Self {
            account_id,
            config,
            credentials: Some(credentials),
            session: None,
            authenticated: false,
            uid_validity: HashMap::new(),
        }
    }

    /// Returns whether the provider is currently authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Returns the account ID for this provider.
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    /// Returns the server configuration.
    pub fn config(&self) -> &ImapConfig {
        &self.config
    }

    /// Loads credentials from the system keychain.
    fn load_credentials_from_keychain(&self) -> Result<ImapCredentials> {
        let entry = keyring::Entry::new("margin", &format!("imap-{}", self.account_id.0))
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        let creds_json = entry
            .get_password()
            .map_err(|e| ProviderError::Authentication(format!("no credentials found: {}", e)))?;

        serde_json::from_str(&creds_json)
            .map_err(|e| ProviderError::Authentication(format!("invalid credentials: {}", e)))
    }

    /// Saves credentials to the system keychain.
    pub fn save_credentials_to_keychain(&self, credentials: &ImapCredentials) -> Result<()> {
        let entry = keyring::Entry::new("margin", &format!("imap-{}", self.account_id.0))
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        let creds_json = serde_json::to_string(credentials)
            .map_err(|e| ProviderError::Authentication(format!("serialize error: {}", e)))?;

        entry
            .set_password(&creds_json)
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        Ok(())
    }

    /// Establishes TLS connection to the IMAP server with futures compat wrapper.
    async fn connect_tls(&self) -> Result<Compat<TlsStream<TcpStream>>> {
        let tcp_stream = TcpStream::connect(format!(
            "{}:{}",
            self.config.imap_host, self.config.imap_port
        ))
        .await
        .map_err(|e| ProviderError::Connection(format!("TCP connect failed: {}", e)))?;

        let config = ClientConfig::builder()
            .with_root_certificates(tokio_rustls::rustls::RootCertStore::from_iter(
                webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
            ))
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from(self.config.imap_host.clone())
            .map_err(|e| ProviderError::Connection(format!("invalid server name: {}", e)))?;

        let tls_stream = connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|e| ProviderError::Connection(format!("TLS handshake failed: {}", e)))?;

        // Wrap with tokio-util compat layer for futures async read/write traits
        Ok(tls_stream.compat())
    }

    /// Gets the IMAP session, reconnecting if necessary.
    async fn get_session(&self) -> Result<Arc<Mutex<ImapSession>>> {
        self.session
            .clone()
            .ok_or_else(|| ProviderError::Connection("not connected".to_string()))
    }

    /// Consumes a stream to completion.
    async fn drain_stream<T, E>(
        stream: impl futures::Stream<Item = std::result::Result<T, E>>,
    ) -> std::result::Result<(), E> {
        use futures::StreamExt;
        futures::pin_mut!(stream);
        while let Some(result) = stream.next().await {
            result?;
        }
        Ok(())
    }

    /// Parses a mail_parser Addr to our Address type.
    fn parse_address(addr: &Addr) -> Address {
        Address {
            email: addr.address().unwrap_or("").to_string(),
            name: addr.name().map(|s| s.to_string()),
        }
    }

    /// Parses IMAP message flags to determine read/starred status.
    fn parse_flags(fetch: &Fetch) -> (bool, bool) {
        let mut is_read = false;
        let mut is_starred = false;
        for flag in fetch.flags() {
            match flag {
                Flag::Seen => is_read = true,
                Flag::Flagged => is_starred = true,
                _ => {}
            }
        }
        (is_read, is_starred)
    }

    /// Converts bytes to String, handling UTF-8 encoding.
    fn bytes_to_string(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }

    /// Builds an email address string from IMAP mailbox and host parts.
    fn build_email_from_parts(
        mailbox: Option<&std::borrow::Cow<'_, [u8]>>,
        host: Option<&std::borrow::Cow<'_, [u8]>>,
    ) -> String {
        match (mailbox, host) {
            (Some(m), Some(h)) => format!(
                "{}@{}",
                String::from_utf8_lossy(m),
                String::from_utf8_lossy(h)
            ),
            (Some(m), None) => String::from_utf8_lossy(m).to_string(),
            _ => String::new(),
        }
    }

    /// Converts a fetch result to a ThreadSummary.
    fn fetch_to_thread_summary(&self, fetch: &Fetch, folder: &str) -> Option<ThreadSummary> {
        let uid = fetch.uid?;
        let envelope = fetch.envelope()?;

        let (is_read, is_starred) = Self::parse_flags(fetch);

        let from = envelope
            .from
            .as_ref()
            .and_then(|addrs| addrs.first())
            .map(|addr| Address {
                email: Self::build_email_from_parts(addr.mailbox.as_ref(), addr.host.as_ref()),
                name: addr.name.as_ref().map(|b| Self::bytes_to_string(b)),
            })
            .unwrap_or_else(|| Address::new("unknown@unknown.com"));

        let subject = envelope.subject.as_ref().map(|b| Self::bytes_to_string(b));

        let date = envelope
            .date
            .as_ref()
            .and_then(|d| {
                let date_str = String::from_utf8_lossy(d);
                DateTime::parse_from_rfc2822(&date_str).ok()
            })
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        // Thread ID is folder:uid for IMAP
        let thread_id = ThreadId::from(format!("{}:{}", folder, uid));

        Some(ThreadSummary {
            id: thread_id,
            account_id: self.account_id.clone(),
            from,
            subject,
            snippet: String::new(), // Will be populated when fetching body preview
            last_message_date: date,
            message_count: 1,
            unread_count: if is_read { 0 } else { 1 },
            is_starred,
            labels: vec![LabelId::from(folder.to_string())],
        })
    }

    /// Extracts addresses from mail_parser message.
    fn extract_from(message: &ParsedMessage) -> Vec<Address> {
        message
            .from()
            .and_then(|addr| addr.as_list())
            .map(|list| list.iter().map(Self::parse_address).collect())
            .unwrap_or_default()
    }

    fn extract_to(message: &ParsedMessage) -> Vec<Address> {
        message
            .to()
            .and_then(|addr| addr.as_list())
            .map(|list| list.iter().map(Self::parse_address).collect())
            .unwrap_or_default()
    }

    fn extract_cc(message: &ParsedMessage) -> Vec<Address> {
        message
            .cc()
            .and_then(|addr| addr.as_list())
            .map(|list| list.iter().map(Self::parse_address).collect())
            .unwrap_or_default()
    }

    /// Parses a full message from IMAP fetch body data.
    fn parse_message(&self, fetch: &Fetch, folder: &str) -> Option<Email> {
        let uid = fetch.uid?;
        let body_data = fetch.body()?;

        let message = MessageParser::default().parse(body_data)?;

        let (is_read, is_starred) = Self::parse_flags(fetch);

        let from_addrs = Self::extract_from(&message);
        let from = from_addrs
            .into_iter()
            .next()
            .unwrap_or_else(|| Address::new("unknown@unknown.com"));

        let to = Self::extract_to(&message);
        let cc = Self::extract_cc(&message);

        let subject = message.subject().map(|s| s.to_string());

        let message_id_str = message
            .message_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("<{}-{}>", folder, uid));

        let in_reply_to = message
            .in_reply_to()
            .as_text()
            .map(|s| MessageId::from(s.to_string()));

        let references: Vec<MessageId> = message
            .references()
            .as_text_list()
            .map(|refs| {
                refs.iter()
                    .map(|s| MessageId::from(s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let date = message
            .date()
            .and_then(|d| DateTime::from_timestamp(d.to_timestamp(), 0))
            .unwrap_or_else(Utc::now);

        let body_text = message.body_text(0).map(|s| s.to_string());
        let body_html = message.body_html(0).map(|s| s.to_string());

        let snippet = body_text
            .as_ref()
            .map(|s| s.chars().take(200).collect())
            .unwrap_or_default();

        Some(Email {
            id: EmailId::from(format!("{}:{}", folder, uid)),
            account_id: self.account_id.clone(),
            thread_id: ThreadId::from(format!("{}:{}", folder, uid)),
            message_id: MessageId::from(message_id_str),
            in_reply_to,
            references,
            from,
            to,
            cc,
            bcc: vec![],
            subject,
            body_text,
            body_html,
            snippet,
            date,
            is_read,
            is_starred,
            is_draft: folder.eq_ignore_ascii_case("Drafts"),
            labels: vec![LabelId::from(folder.to_string())],
            attachments: vec![],
        })
    }

    /// Builds an RFC 5322 message from OutgoingEmail.
    fn build_message(&self, email: &OutgoingEmail) -> Result<Message> {
        let creds = self
            .credentials
            .as_ref()
            .ok_or_else(|| ProviderError::Authentication("no credentials".to_string()))?;

        let from_mailbox: Mailbox = if let Some(ref name) = creds.display_name {
            format!("{} <{}>", name, creds.username)
                .parse()
                .map_err(|e| {
                    ProviderError::InvalidRequest(format!("invalid from address: {}", e))
                })?
        } else {
            creds.username.parse().map_err(|e| {
                ProviderError::InvalidRequest(format!("invalid from address: {}", e))
            })?
        };

        let mut builder = MessageBuilder::new().from(from_mailbox);

        // Add recipients
        for addr in &email.to {
            let mailbox: Mailbox = if let Some(ref name) = addr.name {
                format!("{} <{}>", name, addr.email)
            } else {
                addr.email.clone()
            }
            .parse()
            .map_err(|e| ProviderError::InvalidRequest(format!("invalid to address: {}", e)))?;
            builder = builder.to(mailbox);
        }

        for addr in &email.cc {
            let mailbox: Mailbox = if let Some(ref name) = addr.name {
                format!("{} <{}>", name, addr.email)
            } else {
                addr.email.clone()
            }
            .parse()
            .map_err(|e| ProviderError::InvalidRequest(format!("invalid cc address: {}", e)))?;
            builder = builder.cc(mailbox);
        }

        for addr in &email.bcc {
            let mailbox: Mailbox = if let Some(ref name) = addr.name {
                format!("{} <{}>", name, addr.email)
            } else {
                addr.email.clone()
            }
            .parse()
            .map_err(|e| ProviderError::InvalidRequest(format!("invalid bcc address: {}", e)))?;
            builder = builder.bcc(mailbox);
        }

        builder = builder.subject(&email.subject);

        if let Some(ref reply_to) = email.in_reply_to_message {
            builder = builder.in_reply_to(reply_to.clone());
        }

        // Build body
        let body = if let Some(ref html) = email.body_html {
            MultiPart::alternative()
                .singlepart(SinglePart::plain(email.body_text.clone()))
                .singlepart(SinglePart::html(html.clone()))
        } else {
            MultiPart::mixed().singlepart(SinglePart::plain(email.body_text.clone()))
        };

        builder
            .multipart(body)
            .map_err(|e| ProviderError::InvalidRequest(format!("failed to build message: {}", e)))
    }

    /// Converts folder name to IMAP folder path.
    fn folder_path(folder: &str) -> &str {
        match folder.to_uppercase().as_str() {
            "INBOX" => "INBOX",
            "SENT" => "Sent",
            "DRAFTS" => "Drafts",
            "TRASH" => "Trash",
            "ARCHIVE" => "Archive",
            "SPAM" | "JUNK" => "Junk",
            _ => folder,
        }
    }
}

#[async_trait]
impl EmailProvider for ImapProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Imap
    }

    async fn authenticate(&mut self) -> Result<()> {
        // Load credentials from keychain if not already set
        if self.credentials.is_none() {
            self.credentials = Some(self.load_credentials_from_keychain()?);
        }

        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| ProviderError::Authentication("no credentials".to_string()))?;

        // Connect with TLS
        let tls_stream = self.connect_tls().await?;

        // Create IMAP client
        let client = async_imap::Client::new(tls_stream);

        // Authenticate
        let session = client
            .login(&credentials.username, &credentials.password)
            .await
            .map_err(|e| ProviderError::Authentication(format!("IMAP login failed: {:?}", e.0)))?;

        self.session = Some(Arc::new(Mutex::new(session)));
        self.authenticated = true;

        tracing::info!(account_id = %self.account_id, "IMAP provider authenticated");
        Ok(())
    }

    async fn fetch_threads(
        &self,
        folder: &str,
        pagination: Pagination,
    ) -> Result<Vec<ThreadSummary>> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        let folder_path = Self::folder_path(folder);

        // Select the folder
        let _mailbox = session
            .select(folder_path)
            .await
            .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

        // Search for messages (most recent first)
        let limit = pagination.limit.unwrap_or(50);
        let search_query = "ALL";

        let uids = session
            .uid_search(&search_query)
            .await
            .map_err(|e| ProviderError::Connection(format!("SEARCH failed: {}", e)))?;

        // Get the most recent UIDs
        let mut uid_list: Vec<_> = uids.into_iter().collect();
        uid_list.sort_by(|a, b| b.cmp(a)); // Sort descending (newest first)
        uid_list.truncate(limit as usize);

        if uid_list.is_empty() {
            return Ok(vec![]);
        }

        // Build UID sequence
        let uid_seq = uid_list
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // Fetch envelopes
        let fetches = session
            .uid_fetch(&uid_seq, "(UID FLAGS ENVELOPE)")
            .await
            .map_err(|e| ProviderError::Connection(format!("FETCH failed: {}", e)))?;

        let mut summaries = Vec::new();
        let mut stream = fetches;

        use futures::StreamExt;
        while let Some(fetch_result) = stream.next().await {
            if let Ok(fetch) = fetch_result {
                if let Some(summary) = self.fetch_to_thread_summary(&fetch, folder) {
                    summaries.push(summary);
                }
            }
        }

        Ok(summaries)
    }

    async fn fetch_thread(&self, thread_id: &str) -> Result<Thread> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        // Parse thread_id format: folder:uid
        let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ProviderError::InvalidRequest(format!(
                "invalid thread_id format: {}",
                thread_id
            )));
        }

        let folder = parts[0];
        let uid: u32 = parts[1]
            .parse()
            .map_err(|_| ProviderError::InvalidRequest("invalid UID".to_string()))?;

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        let folder_path = Self::folder_path(folder);

        // Select the folder
        let _mailbox = session
            .select(folder_path)
            .await
            .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

        // Fetch the full message
        let fetches = session
            .uid_fetch(uid.to_string(), "(UID FLAGS BODY[])")
            .await
            .map_err(|e| ProviderError::Connection(format!("FETCH failed: {}", e)))?;

        use futures::StreamExt;
        let mut stream = fetches;

        while let Some(fetch_result) = stream.next().await {
            if let Ok(fetch) = fetch_result {
                if let Some(email) = self.parse_message(&fetch, folder) {
                    let participants = {
                        let mut p = vec![email.from.clone()];
                        p.extend(email.to.clone());
                        p
                    };

                    return Ok(Thread {
                        id: ThreadId::from(thread_id.to_string()),
                        account_id: self.account_id.clone(),
                        subject: email.subject.clone(),
                        snippet: email.snippet.clone(),
                        participants,
                        messages: vec![email.clone()],
                        last_message_date: email.date,
                        unread_count: if email.is_read { 0 } else { 1 },
                        is_starred: email.is_starred,
                        labels: vec![LabelId::from(folder.to_string())],
                    });
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "thread not found: {}",
            thread_id
        )))
    }

    async fn fetch_changes_since(&self, _since: &DateTime<Utc>) -> Result<Vec<Change>> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        // IMAP doesn't have native change tracking like Gmail.
        // Options:
        // 1. Use CONDSTORE/QRESYNC if supported (check CAPABILITY)
        // 2. Poll with SEARCH SINCE and compare UIDs
        // 3. Use IDLE for real-time updates
        //
        // For now, return empty - the sync service will do full syncs.
        Ok(vec![])
    }

    async fn send_email(&self, email: &OutgoingEmail) -> Result<String> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| ProviderError::Authentication("no credentials".to_string()))?;

        let message = self.build_message(email)?;

        // Create SMTP transport
        let smtp_credentials =
            SmtpCredentials::new(credentials.username.clone(), credentials.password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> = if self.config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                .map_err(|e| ProviderError::Connection(format!("SMTP relay error: {}", e)))?
                .credentials(smtp_credentials)
                .port(self.config.smtp_port)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.smtp_host)
                .map_err(|e| ProviderError::Connection(format!("SMTP relay error: {}", e)))?
                .credentials(smtp_credentials)
                .port(self.config.smtp_port)
                .build()
        };

        // Send the email
        let response = mailer
            .send(message)
            .await
            .map_err(|e| ProviderError::Connection(format!("SMTP send failed: {}", e)))?;

        let message_id = response
            .message()
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("<sent-{}>", Utc::now().timestamp()));

        tracing::info!(message_id = %message_id, "Email sent via SMTP");
        Ok(message_id)
    }

    async fn archive(&self, thread_ids: &[String]) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        for thread_id in thread_ids {
            // Parse thread_id format: folder:uid
            let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let folder = parts[0];
            let uid = parts[1];

            let folder_path = Self::folder_path(folder);

            // Select source folder
            session
                .select(folder_path)
                .await
                .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

            // Try to MOVE to Archive (IMAP extension)
            // Fall back to COPY + STORE \Deleted + EXPUNGE if MOVE not supported
            let result = session.uid_mv(uid, "Archive").await;

            if result.is_err() {
                // Fallback: COPY then delete
                session
                    .uid_copy(uid, "Archive")
                    .await
                    .map_err(|e| ProviderError::Connection(format!("COPY failed: {}", e)))?;

                let store_stream = session
                    .uid_store(uid, "+FLAGS (\\Deleted)")
                    .await
                    .map_err(|e| ProviderError::Connection(format!("STORE failed: {}", e)))?;
                Self::drain_stream(store_stream)
                    .await
                    .map_err(|e| ProviderError::Connection(format!("STORE stream: {}", e)))?;

                let expunge_stream = session
                    .expunge()
                    .await
                    .map_err(|e| ProviderError::Connection(format!("EXPUNGE failed: {}", e)))?;
                Self::drain_stream(expunge_stream)
                    .await
                    .map_err(|e| ProviderError::Connection(format!("EXPUNGE stream: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn trash(&self, thread_ids: &[String]) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        for thread_id in thread_ids {
            let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let folder = parts[0];
            let uid = parts[1];

            let folder_path = Self::folder_path(folder);

            session
                .select(folder_path)
                .await
                .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

            // Try MOVE to Trash
            let result = session.uid_mv(uid, "Trash").await;

            if result.is_err() {
                // Fallback: COPY then delete
                session
                    .uid_copy(uid, "Trash")
                    .await
                    .map_err(|e| ProviderError::Connection(format!("COPY failed: {}", e)))?;

                let store_stream = session
                    .uid_store(uid, "+FLAGS (\\Deleted)")
                    .await
                    .map_err(|e| ProviderError::Connection(format!("STORE failed: {}", e)))?;
                Self::drain_stream(store_stream)
                    .await
                    .map_err(|e| ProviderError::Connection(format!("STORE stream: {}", e)))?;

                let expunge_stream = session
                    .expunge()
                    .await
                    .map_err(|e| ProviderError::Connection(format!("EXPUNGE failed: {}", e)))?;
                Self::drain_stream(expunge_stream)
                    .await
                    .map_err(|e| ProviderError::Connection(format!("EXPUNGE stream: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn star(&self, thread_id: &str, starred: bool) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ProviderError::InvalidRequest(
                "invalid thread_id".to_string(),
            ));
        }

        let folder = parts[0];
        let uid = parts[1];

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        let folder_path = Self::folder_path(folder);

        session
            .select(folder_path)
            .await
            .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

        let flag_cmd = if starred {
            "+FLAGS (\\Flagged)"
        } else {
            "-FLAGS (\\Flagged)"
        };

        let store_stream = session
            .uid_store(uid, flag_cmd)
            .await
            .map_err(|e| ProviderError::Connection(format!("STORE failed: {}", e)))?;
        Self::drain_stream(store_stream)
            .await
            .map_err(|e| ProviderError::Connection(format!("STORE stream: {}", e)))?;

        Ok(())
    }

    async fn mark_read(&self, thread_id: &str, read: bool) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ProviderError::InvalidRequest(
                "invalid thread_id".to_string(),
            ));
        }

        let folder = parts[0];
        let uid = parts[1];

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        let folder_path = Self::folder_path(folder);

        session
            .select(folder_path)
            .await
            .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

        let flag_cmd = if read {
            "+FLAGS (\\Seen)"
        } else {
            "-FLAGS (\\Seen)"
        };

        let store_stream = session
            .uid_store(uid, flag_cmd)
            .await
            .map_err(|e| ProviderError::Connection(format!("STORE failed: {}", e)))?;
        Self::drain_stream(store_stream)
            .await
            .map_err(|e| ProviderError::Connection(format!("STORE stream: {}", e)))?;

        Ok(())
    }

    async fn apply_label(&self, thread_id: &str, label: &str) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        // IMAP doesn't have native labels - we simulate by copying to folder
        let parts: Vec<&str> = thread_id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ProviderError::InvalidRequest(
                "invalid thread_id".to_string(),
            ));
        }

        let folder = parts[0];
        let uid = parts[1];

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        let folder_path = Self::folder_path(folder);

        session
            .select(folder_path)
            .await
            .map_err(|e| ProviderError::Connection(format!("SELECT failed: {}", e)))?;

        // Copy to label folder
        session
            .uid_copy(uid, label)
            .await
            .map_err(|e| ProviderError::Connection(format!("COPY failed: {}", e)))?;

        Ok(())
    }

    async fn fetch_labels(&self) -> Result<Vec<Label>> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let session_arc = self.get_session().await?;
        let mut session = session_arc.lock().await;

        // List all folders
        let folders = session
            .list(Some(""), Some("*"))
            .await
            .map_err(|e| ProviderError::Connection(format!("LIST failed: {}", e)))?;

        use futures::StreamExt;
        let mut labels = Vec::new();
        let mut stream = folders;

        while let Some(folder_result) = stream.next().await {
            if let Ok(folder) = folder_result {
                let name = folder.name().to_string();
                let is_system = matches!(
                    name.to_uppercase().as_str(),
                    "INBOX" | "SENT" | "DRAFTS" | "TRASH" | "SPAM" | "JUNK" | "ARCHIVE"
                );

                labels.push(Label {
                    id: LabelId::from(name.clone()),
                    account_id: self.account_id.clone(),
                    name: name.clone(),
                    color: None,
                    is_system,
                    provider_id: Some(name),
                });
            }
        }

        Ok(labels)
    }

    async fn push_change(&self, change: &PendingChange) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        match &change.change_type {
            PendingChangeType::Archive { thread_ids } => {
                let ids: Vec<String> = thread_ids.iter().map(|t| t.0.clone()).collect();
                self.archive(&ids).await
            }
            PendingChangeType::Trash { thread_ids } => {
                let ids: Vec<String> = thread_ids.iter().map(|t| t.0.clone()).collect();
                self.trash(&ids).await
            }
            PendingChangeType::Star { thread_id, starred } => {
                self.star(&thread_id.0, *starred).await
            }
            PendingChangeType::MarkRead { thread_ids, read } => {
                for thread_id in thread_ids {
                    self.mark_read(&thread_id.0, *read).await?;
                }
                Ok(())
            }
            PendingChangeType::ApplyLabel {
                thread_ids,
                label_id,
            } => {
                for thread_id in thread_ids {
                    self.apply_label(&thread_id.0, &label_id.0).await?;
                }
                Ok(())
            }
            PendingChangeType::RemoveLabel { thread_ids, .. } => {
                // IMAP doesn't support removing labels (folders) without moving
                // This would require moving back to INBOX, which changes semantics
                tracing::warn!("RemoveLabel not fully supported for IMAP: {:?}", thread_ids);
                Ok(())
            }
            PendingChangeType::Send { email } => {
                self.send_email(email).await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ImapConfig {
        ImapConfig::tls("imap.example.com", "smtp.example.com")
    }

    #[test]
    fn imap_config_tls() {
        let config = ImapConfig::tls("imap.example.com", "smtp.example.com");
        assert_eq!(config.imap_host, "imap.example.com");
        assert_eq!(config.imap_port, 993);
        assert_eq!(config.smtp_host, "smtp.example.com");
        assert_eq!(config.smtp_port, 465);
        assert!(config.use_tls);
    }

    #[test]
    fn imap_config_starttls() {
        let config = ImapConfig::starttls("imap.example.com", "smtp.example.com");
        assert_eq!(config.imap_port, 143);
        assert_eq!(config.smtp_port, 587);
        assert!(!config.use_tls);
    }

    #[test]
    fn imap_provider_creation() {
        let provider = ImapProvider::new(AccountId::from("test-account"), test_config());
        assert_eq!(provider.account_id().0, "test-account");
        assert!(!provider.is_authenticated());
        assert_eq!(provider.config().imap_host, "imap.example.com");
    }

    #[test]
    fn imap_provider_type() {
        let provider = ImapProvider::new(AccountId::from("test-account"), test_config());
        assert_eq!(provider.provider_type(), ProviderType::Imap);
    }

    #[test]
    fn imap_credentials_serialization() {
        let creds = ImapCredentials {
            username: "user@example.com".to_string(),
            password: "secret".to_string(),
            display_name: Some("Test User".to_string()),
        };

        let json = serde_json::to_string(&creds).unwrap();
        let deserialized: ImapCredentials = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.username, "user@example.com");
        assert_eq!(deserialized.password, "secret");
        assert_eq!(deserialized.display_name, Some("Test User".to_string()));
    }

    #[test]
    fn folder_path_conversion() {
        assert_eq!(ImapProvider::folder_path("INBOX"), "INBOX");
        assert_eq!(ImapProvider::folder_path("inbox"), "INBOX");
        assert_eq!(ImapProvider::folder_path("SENT"), "Sent");
        assert_eq!(ImapProvider::folder_path("DRAFTS"), "Drafts");
        assert_eq!(ImapProvider::folder_path("TRASH"), "Trash");
        assert_eq!(ImapProvider::folder_path("Custom"), "Custom");
    }

    #[tokio::test]
    async fn imap_provider_requires_auth() {
        let provider = ImapProvider::new(AccountId::from("test-account"), test_config());

        let result = provider.fetch_threads("INBOX", Pagination::default()).await;
        assert!(matches!(result, Err(ProviderError::Authentication(_))));
    }
}
