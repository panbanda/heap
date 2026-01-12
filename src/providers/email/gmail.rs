//! Gmail API provider implementation.
//!
//! This module provides an [`EmailProvider`] implementation using the Gmail REST API.
//! It handles OAuth 2.0 authentication, fetching emails via the Gmail API, and
//! sending emails via the Gmail API.
//!
//! # Authentication
//!
//! Gmail uses OAuth 2.0 for authentication. Access tokens and refresh tokens are
//! stored in the system keychain, referenced by account ID. The provider handles
//! token refresh automatically when tokens expire.
//!
//! # API Usage
//!
//! This provider uses the Gmail API v1:
//! - `users.threads.list` for fetching thread summaries
//! - `users.threads.get` for fetching complete threads
//! - `users.history.list` for incremental sync
//! - `users.messages.send` for sending emails
//! - `users.labels.list` for fetching labels

use async_trait::async_trait;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use super::{
    Change, EmailProvider, EmailUpdate, NewEmailData, OutgoingEmail, Pagination, PendingChange,
    PendingChangeType, ProviderError, Result,
};
use crate::domain::{
    AccountId, Address, Email, EmailId, Label, LabelId, MessageId, ProviderType, Thread, ThreadId,
    ThreadSummary,
};

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Gmail API thread list response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadListResponse {
    threads: Option<Vec<GmailThread>>,
    #[allow(dead_code)]
    next_page_token: Option<String>,
    #[allow(dead_code)]
    result_size_estimate: Option<u32>,
}

/// Gmail API thread.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailThread {
    id: String,
    #[allow(dead_code)]
    history_id: Option<String>,
    messages: Option<Vec<GmailMessage>>,
    snippet: Option<String>,
}

/// Gmail API message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailMessage {
    id: String,
    thread_id: String,
    label_ids: Option<Vec<String>>,
    snippet: Option<String>,
    payload: Option<GmailMessagePayload>,
    internal_date: Option<String>,
    #[allow(dead_code)]
    size_estimate: Option<u32>,
}

/// Gmail message payload (headers and body parts).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailMessagePayload {
    headers: Option<Vec<GmailHeader>>,
    parts: Option<Vec<GmailPart>>,
    body: Option<GmailBody>,
    #[allow(dead_code)]
    mime_type: Option<String>,
}

/// Gmail message header.
#[derive(Debug, Deserialize)]
struct GmailHeader {
    name: String,
    value: String,
}

/// Gmail message part (for multipart messages).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailPart {
    mime_type: Option<String>,
    body: Option<GmailBody>,
    parts: Option<Vec<GmailPart>>,
    #[allow(dead_code)]
    filename: Option<String>,
}

/// Gmail message body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailBody {
    data: Option<String>,
    #[allow(dead_code)]
    size: Option<u32>,
    #[allow(dead_code)]
    attachment_id: Option<String>,
}

/// Gmail API label.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailLabel {
    id: String,
    name: String,
    #[serde(rename = "type")]
    label_type: Option<String>,
    #[allow(dead_code)]
    message_list_visibility: Option<String>,
    #[allow(dead_code)]
    label_list_visibility: Option<String>,
}

/// Gmail labels list response.
#[derive(Debug, Deserialize)]
struct LabelsListResponse {
    labels: Option<Vec<GmailLabel>>,
}

/// Gmail history list response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryListResponse {
    history: Option<Vec<GmailHistory>>,
    #[allow(dead_code)]
    next_page_token: Option<String>,
    #[allow(dead_code)]
    history_id: Option<String>,
}

/// Gmail history record.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailHistory {
    messages_added: Option<Vec<GmailHistoryMessage>>,
    messages_deleted: Option<Vec<GmailHistoryMessage>>,
    labels_added: Option<Vec<GmailHistoryLabelChange>>,
    labels_removed: Option<Vec<GmailHistoryLabelChange>>,
}

/// Gmail history message reference.
#[derive(Debug, Deserialize)]
struct GmailHistoryMessage {
    message: GmailHistoryMessageRef,
}

/// Gmail history message ref.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailHistoryMessageRef {
    id: String,
    #[allow(dead_code)]
    thread_id: String,
}

/// Gmail history label change.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailHistoryLabelChange {
    message: GmailHistoryMessageRef,
    label_ids: Vec<String>,
}

/// Gmail modify request body.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModifyRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    add_label_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    remove_label_ids: Vec<String>,
}

/// OAuth token response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
}

/// OAuth credentials stored in keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailCredentials {
    /// OAuth refresh token.
    pub refresh_token: String,
    /// OAuth client ID.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
}

/// Gmail API provider.
///
/// Implements [`EmailProvider`] using the Gmail REST API with OAuth 2.0 authentication.
///
/// # Example
///
/// ```ignore
/// use margin::providers::email::{GmailProvider, EmailProvider, Pagination};
///
/// let mut provider = GmailProvider::new(account_id);
/// provider.authenticate().await?;
///
/// let threads = provider.fetch_threads("INBOX", Pagination::with_limit(50)).await?;
/// ```
pub struct GmailProvider {
    /// Account ID for keychain credential lookup.
    account_id: AccountId,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// OAuth credentials.
    credentials: Option<GmailCredentials>,
    /// Current OAuth access token (refreshed as needed).
    access_token: Option<String>,
    /// Whether the provider is authenticated.
    authenticated: bool,
    /// Last known history ID for incremental sync.
    last_history_id: Option<String>,
}

impl GmailProvider {
    /// Creates a new Gmail provider for the specified account.
    ///
    /// The provider is not authenticated until [`authenticate`](Self::authenticate) is called.
    ///
    /// # Arguments
    ///
    /// * `account_id` - Account ID used to look up OAuth tokens in the keychain
    pub fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            client: reqwest::Client::new(),
            credentials: None,
            access_token: None,
            authenticated: false,
            last_history_id: None,
        }
    }

    /// Creates a new Gmail provider with explicit credentials (for testing or direct use).
    pub fn with_credentials(account_id: AccountId, credentials: GmailCredentials) -> Self {
        Self {
            account_id,
            client: reqwest::Client::new(),
            credentials: Some(credentials),
            access_token: None,
            authenticated: false,
            last_history_id: None,
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

    /// Loads credentials from the system keychain.
    fn load_credentials_from_keychain(&self) -> Result<GmailCredentials> {
        let entry = keyring::Entry::new("margin", &format!("gmail-{}", self.account_id.0))
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        let creds_json = entry
            .get_password()
            .map_err(|e| ProviderError::Authentication(format!("no credentials found: {}", e)))?;

        serde_json::from_str(&creds_json)
            .map_err(|e| ProviderError::Authentication(format!("invalid credentials: {}", e)))
    }

    /// Saves credentials to the system keychain.
    pub fn save_credentials_to_keychain(&self, credentials: &GmailCredentials) -> Result<()> {
        let entry = keyring::Entry::new("margin", &format!("gmail-{}", self.account_id.0))
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        let creds_json = serde_json::to_string(credentials)
            .map_err(|e| ProviderError::Authentication(format!("serialize error: {}", e)))?;

        entry
            .set_password(&creds_json)
            .map_err(|e| ProviderError::Authentication(format!("keyring error: {}", e)))?;

        Ok(())
    }

    /// Refreshes the OAuth access token using the refresh token.
    async fn refresh_access_token(&mut self) -> Result<String> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| ProviderError::Authentication("no credentials available".to_string()))?;

        let params = [
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Authentication(format!(
                "token refresh failed ({}): {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("parse token response: {}", e)))?;

        self.access_token = Some(token_response.access_token.clone());
        Ok(token_response.access_token)
    }

    /// Builds authorization headers for API requests.
    fn auth_headers(&self) -> Result<HeaderMap> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| ProviderError::Authentication("not authenticated".to_string()))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| ProviderError::Internal(format!("invalid header: {}", e)))?,
        );
        Ok(headers)
    }

    /// Makes an authenticated GET request to the Gmail API.
    async fn get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", GMAIL_API_BASE, endpoint);
        let headers = self.auth_headers()?;

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Makes an authenticated POST request to the Gmail API.
    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", GMAIL_API_BASE, endpoint);
        let mut headers = self.auth_headers()?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Makes an authenticated POST request that doesn't return a body.
    async fn post_no_response<B: Serialize>(&self, endpoint: &str, body: &B) -> Result<()> {
        let url = format!("{}{}", GMAIL_API_BASE, endpoint);
        let mut headers = self.auth_headers()?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }
        Ok(())
    }

    /// Handles API response, checking for errors.
    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("parse response: {}", e)))
    }

    /// Handles API error responses.
    async fn handle_error(&self, response: reqwest::Response) -> ProviderError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 => ProviderError::Authentication(format!("unauthorized: {}", body)),
            404 => ProviderError::NotFound(body),
            429 => ProviderError::RateLimited {
                retry_after_secs: None,
            },
            _ => ProviderError::Internal(format!("API error ({}): {}", status, body)),
        }
    }

    /// Converts a Gmail label ID to a folder name for querying.
    fn label_id_for_folder(folder: &str) -> &str {
        match folder.to_uppercase().as_str() {
            "INBOX" => "INBOX",
            "STARRED" => "STARRED",
            "SENT" => "SENT",
            "DRAFTS" => "DRAFT",
            "TRASH" => "TRASH",
            "SPAM" => "SPAM",
            "ARCHIVE" | "ALL" => "all", // Special: no INBOX label
            _ => folder,
        }
    }

    /// Parses an email address from a header value like "Name <email@example.com>".
    fn parse_address(value: &str) -> Address {
        let value = value.trim();
        if let Some(start) = value.find('<') {
            if let Some(end) = value.find('>') {
                let email = value[start + 1..end].trim().to_string();
                let name = value[..start].trim().trim_matches('"').to_string();
                return Address {
                    email,
                    name: if name.is_empty() { None } else { Some(name) },
                };
            }
        }
        Address {
            email: value.to_string(),
            name: None,
        }
    }

    /// Parses multiple addresses from a comma-separated header value.
    fn parse_addresses(value: &str) -> Vec<Address> {
        value
            .split(',')
            .map(|s| Self::parse_address(s.trim()))
            .collect()
    }

    /// Extracts the body text from a Gmail message.
    fn extract_body(payload: &GmailMessagePayload) -> (Option<String>, Option<String>) {
        let mut text = None;
        let mut html = None;

        // Check direct body
        if let Some(body) = &payload.body {
            if let Some(data) = &body.data {
                if let Ok(decoded) = BASE64_URL_SAFE_NO_PAD.decode(data) {
                    if let Ok(s) = String::from_utf8(decoded) {
                        text = Some(s);
                    }
                }
            }
        }

        // Check parts for multipart messages
        if let Some(parts) = &payload.parts {
            Self::extract_body_from_parts(parts, &mut text, &mut html);
        }

        (text, html)
    }

    /// Recursively extracts body from message parts.
    fn extract_body_from_parts(
        parts: &[GmailPart],
        text: &mut Option<String>,
        html: &mut Option<String>,
    ) {
        for part in parts {
            let mime = part.mime_type.as_deref().unwrap_or("");

            if mime == "text/plain" && text.is_none() {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        if let Ok(decoded) = BASE64_URL_SAFE_NO_PAD.decode(data) {
                            if let Ok(s) = String::from_utf8(decoded) {
                                *text = Some(s);
                            }
                        }
                    }
                }
            } else if mime == "text/html" && html.is_none() {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        if let Ok(decoded) = BASE64_URL_SAFE_NO_PAD.decode(data) {
                            if let Ok(s) = String::from_utf8(decoded) {
                                *html = Some(s);
                            }
                        }
                    }
                }
            }

            // Recurse into nested parts
            if let Some(nested) = &part.parts {
                Self::extract_body_from_parts(nested, text, html);
            }
        }
    }

    /// Converts a Gmail message to our domain Email type.
    fn gmail_message_to_email(&self, msg: &GmailMessage) -> Email {
        let payload = msg.payload.as_ref();
        let headers = payload.and_then(|p| p.headers.as_ref());

        let get_header = |name: &str| -> Option<String> {
            headers.and_then(|h| {
                h.iter()
                    .find(|hdr| hdr.name.eq_ignore_ascii_case(name))
                    .map(|hdr| hdr.value.clone())
            })
        };

        let from = get_header("From")
            .map(|v| Self::parse_address(&v))
            .unwrap_or_else(|| Address::new("unknown@unknown.com"));

        let to = get_header("To")
            .map(|v| Self::parse_addresses(&v))
            .unwrap_or_default();

        let cc = get_header("Cc")
            .map(|v| Self::parse_addresses(&v))
            .unwrap_or_default();

        let subject = get_header("Subject");
        let message_id = get_header("Message-ID")
            .map(MessageId::from)
            .unwrap_or_else(|| MessageId::from(format!("<{}>", msg.id)));

        let in_reply_to = get_header("In-Reply-To").map(MessageId::from);
        let references = get_header("References")
            .map(|v| {
                v.split_whitespace()
                    .map(|s| MessageId::from(s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let date = msg
            .internal_date
            .as_ref()
            .and_then(|d| d.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(Utc::now);

        let label_strings = msg.label_ids.clone().unwrap_or_default();
        let is_read = !label_strings.iter().any(|l| l == "UNREAD");
        let is_starred = label_strings.iter().any(|l| l == "STARRED");
        let is_draft = label_strings.iter().any(|l| l == "DRAFT");
        let labels: Vec<LabelId> = label_strings.into_iter().map(LabelId::from).collect();

        let (body_text, body_html) = payload.map(Self::extract_body).unwrap_or((None, None));

        let snippet = msg.snippet.clone().unwrap_or_default();

        Email {
            id: EmailId::from(msg.id.clone()),
            account_id: self.account_id.clone(),
            thread_id: ThreadId::from(msg.thread_id.clone()),
            message_id,
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
            is_draft,
            labels,
            attachments: vec![], // TODO: parse attachments
        }
    }

    /// Builds an RFC 5322 message from OutgoingEmail for sending.
    ///
    /// Note: The `from` address is not part of OutgoingEmail since Gmail's API
    /// uses the authenticated account's address automatically.
    fn build_raw_message(&self, email: &OutgoingEmail, from_address: &str) -> String {
        let mut message = String::new();

        // Headers
        message.push_str(&format!("From: {}\r\n", from_address));

        let to_addrs: Vec<String> = email.to.iter().map(|a| a.email.clone()).collect();
        message.push_str(&format!("To: {}\r\n", to_addrs.join(", ")));

        if !email.cc.is_empty() {
            let cc_addrs: Vec<String> = email.cc.iter().map(|a| a.email.clone()).collect();
            message.push_str(&format!("Cc: {}\r\n", cc_addrs.join(", ")));
        }

        if !email.bcc.is_empty() {
            let bcc_addrs: Vec<String> = email.bcc.iter().map(|a| a.email.clone()).collect();
            message.push_str(&format!("Bcc: {}\r\n", bcc_addrs.join(", ")));
        }

        message.push_str(&format!("Subject: {}\r\n", email.subject));

        if let Some(in_reply_to) = &email.in_reply_to_message {
            message.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
        }

        message.push_str("MIME-Version: 1.0\r\n");
        message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        message.push_str("\r\n");

        // Body
        message.push_str(&email.body_text);

        message
    }
}

#[async_trait]
impl EmailProvider for GmailProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Gmail
    }

    async fn authenticate(&mut self) -> Result<()> {
        // Load credentials from keychain if not already set
        if self.credentials.is_none() {
            self.credentials = Some(self.load_credentials_from_keychain()?);
        }

        // Refresh the access token
        self.refresh_access_token().await?;
        self.authenticated = true;

        tracing::info!(account_id = %self.account_id, "Gmail provider authenticated");
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

        let label_id = Self::label_id_for_folder(folder);

        // Build query parameters
        let limit = pagination.limit.unwrap_or(50);
        let mut endpoint = format!("/threads?labelIds={}&maxResults={}", label_id, limit);
        if let Some(token) = &pagination.page_token {
            endpoint.push_str(&format!("&pageToken={}", token));
        }

        let response: ThreadListResponse = self.get(&endpoint).await?;

        let threads = response.threads.unwrap_or_default();
        let mut summaries = Vec::with_capacity(threads.len());

        for thread in threads {
            // For thread list, we need to fetch minimal details
            // The list response only has id and snippet
            let first_message = thread.messages.as_ref().and_then(|m| m.first());

            let from = first_message
                .and_then(|m| m.payload.as_ref())
                .and_then(|p| p.headers.as_ref())
                .and_then(|h| h.iter().find(|hdr| hdr.name.eq_ignore_ascii_case("From")))
                .map(|h| Self::parse_address(&h.value))
                .unwrap_or_else(|| Address::new("unknown@unknown.com"));

            let subject = first_message
                .and_then(|m| m.payload.as_ref())
                .and_then(|p| p.headers.as_ref())
                .and_then(|h| {
                    h.iter()
                        .find(|hdr| hdr.name.eq_ignore_ascii_case("Subject"))
                })
                .map(|h| h.value.clone());

            let date = first_message
                .and_then(|m| m.internal_date.as_ref())
                .and_then(|d| d.parse::<i64>().ok())
                .and_then(DateTime::from_timestamp_millis)
                .unwrap_or_else(Utc::now);

            let labels = first_message
                .and_then(|m| m.label_ids.as_ref())
                .cloned()
                .unwrap_or_default();

            let unread_count = if labels.iter().any(|l| l == "UNREAD") {
                1
            } else {
                0
            };

            let is_starred = labels.iter().any(|l| l == "STARRED");

            let label_ids: Vec<LabelId> = labels.into_iter().map(LabelId::from).collect();

            summaries.push(ThreadSummary {
                id: ThreadId::from(thread.id.clone()),
                account_id: self.account_id.clone(),
                from,
                subject,
                snippet: thread.snippet.unwrap_or_default(),
                last_message_date: date,
                message_count: thread.messages.map(|m| m.len() as u32).unwrap_or(1),
                unread_count,
                is_starred,
                labels: label_ids,
            });
        }

        Ok(summaries)
    }

    async fn fetch_thread(&self, thread_id: &str) -> Result<Thread> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let endpoint = format!("/threads/{}?format=full", thread_id);
        let response: GmailThread = self.get(&endpoint).await?;

        let messages: Vec<Email> = response
            .messages
            .unwrap_or_default()
            .iter()
            .map(|m| self.gmail_message_to_email(m))
            .collect();

        let subject = messages.first().and_then(|m| m.subject.clone());
        let snippet = messages
            .last()
            .map(|m| m.snippet.clone())
            .unwrap_or_default();
        let last_message_date = messages.last().map(|m| m.date).unwrap_or_else(Utc::now);
        let unread_count = messages.iter().filter(|m| !m.is_read).count() as u32;
        let is_starred = messages.iter().any(|m| m.is_starred);

        // Collect unique participants from all messages
        let mut participants = Vec::new();
        for msg in &messages {
            if !participants
                .iter()
                .any(|p: &Address| p.email == msg.from.email)
            {
                participants.push(msg.from.clone());
            }
            for recipient in &msg.to {
                if !participants.iter().any(|p| p.email == recipient.email) {
                    participants.push(recipient.clone());
                }
            }
        }

        // Collect unique labels from all messages
        let mut label_set = std::collections::HashSet::new();
        for msg in &messages {
            for label in &msg.labels {
                label_set.insert(label.clone());
            }
        }
        let labels: Vec<LabelId> = label_set.into_iter().collect();

        Ok(Thread {
            id: ThreadId::from(response.id),
            account_id: self.account_id.clone(),
            subject,
            snippet,
            participants,
            messages,
            last_message_date,
            unread_count,
            is_starred,
            labels,
        })
    }

    async fn fetch_changes_since(&self, _since: &DateTime<Utc>) -> Result<Vec<Change>> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        // Gmail uses history ID, not timestamp, for incremental sync
        let history_id = match &self.last_history_id {
            Some(id) => id.clone(),
            None => {
                // No history ID means we need a full sync first
                return Ok(vec![]);
            }
        };

        let endpoint = format!(
            "/history?startHistoryId={}&historyTypes=messageAdded,messageDeleted,labelAdded,labelRemoved",
            history_id
        );

        let response: HistoryListResponse = self.get(&endpoint).await?;

        let mut changes = Vec::new();

        if let Some(history) = response.history {
            for record in history {
                // Handle new messages
                if let Some(added) = record.messages_added {
                    for item in added {
                        // Fetch the full message
                        let msg_endpoint = format!("/messages/{}?format=full", item.message.id);
                        if let Ok(msg) = self.get::<GmailMessage>(&msg_endpoint).await {
                            let email = self.gmail_message_to_email(&msg);
                            let new_email = NewEmailData {
                                id: email.id,
                                thread_id: email.thread_id,
                                from: email.from,
                                to: email.to,
                                cc: email.cc,
                                subject: email.subject,
                                snippet: email.snippet,
                                date: email.date,
                                labels: email.labels,
                                is_read: email.is_read,
                                is_starred: email.is_starred,
                                raw: None,
                            };
                            changes.push(Change::NewEmail(new_email));
                        }
                    }
                }

                // Handle deleted messages
                if let Some(deleted) = record.messages_deleted {
                    for item in deleted {
                        changes.push(Change::Deleted(EmailId::from(item.message.id)));
                    }
                }

                // Handle label changes (read/unread, starred, etc.)
                if let Some(label_added) = record.labels_added {
                    for item in label_added {
                        let mut is_starred = None;
                        let mut is_read = None;
                        let mut labels = Vec::new();
                        for label in &item.label_ids {
                            match label.as_str() {
                                "STARRED" => is_starred = Some(true),
                                "UNREAD" => is_read = Some(false),
                                _ => labels.push(LabelId::from(label.clone())),
                            }
                        }
                        let update = EmailUpdate {
                            id: EmailId::from(item.message.id),
                            labels: if labels.is_empty() {
                                None
                            } else {
                                Some(labels)
                            },
                            is_read,
                            is_starred,
                        };
                        changes.push(Change::Updated(update));
                    }
                }

                if let Some(label_removed) = record.labels_removed {
                    for item in label_removed {
                        let mut is_starred = None;
                        let mut is_read = None;
                        for label in &item.label_ids {
                            match label.as_str() {
                                "STARRED" => is_starred = Some(false),
                                "UNREAD" => is_read = Some(true),
                                _ => {}
                            }
                        }
                        let update = EmailUpdate {
                            id: EmailId::from(item.message.id),
                            labels: None, // Can't express "remove these labels" with this schema
                            is_read,
                            is_starred,
                        };
                        changes.push(Change::Updated(update));
                    }
                }
            }
        }

        Ok(changes)
    }

    async fn send_email(&self, email: &OutgoingEmail) -> Result<String> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        // Gmail API sends from the authenticated user; "me" is a special alias
        let from_address = format!("{}@gmail.com", self.account_id.0);
        let raw_message = self.build_raw_message(email, &from_address);
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(raw_message.as_bytes());

        #[derive(Serialize)]
        struct SendRequest {
            raw: String,
        }

        #[derive(Deserialize)]
        struct SendResponse {
            id: String,
        }

        let response: SendResponse = self
            .post("/messages/send", &SendRequest { raw: encoded })
            .await?;

        tracing::info!(message_id = %response.id, "Email sent via Gmail API");
        Ok(response.id)
    }

    async fn archive(&self, thread_ids: &[String]) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        for thread_id in thread_ids {
            let endpoint = format!("/threads/{}/modify", thread_id);
            let body = ModifyRequest {
                add_label_ids: vec![],
                remove_label_ids: vec!["INBOX".to_string()],
            };
            self.post_no_response(&endpoint, &body).await?;
        }

        Ok(())
    }

    async fn trash(&self, thread_ids: &[String]) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        for thread_id in thread_ids {
            let endpoint = format!("/threads/{}/trash", thread_id);
            // Gmail trash endpoint expects POST with empty body
            let url = format!("{}{}", GMAIL_API_BASE, endpoint);
            let headers = self.auth_headers()?;

            let response = self
                .client
                .post(&url)
                .headers(headers)
                .send()
                .await
                .map_err(|e| ProviderError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                return Err(self.handle_error(response).await);
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

        let endpoint = format!("/threads/{}/modify", thread_id);
        let body = if starred {
            ModifyRequest {
                add_label_ids: vec!["STARRED".to_string()],
                remove_label_ids: vec![],
            }
        } else {
            ModifyRequest {
                add_label_ids: vec![],
                remove_label_ids: vec!["STARRED".to_string()],
            }
        };

        self.post_no_response(&endpoint, &body).await
    }

    async fn mark_read(&self, thread_id: &str, read: bool) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let endpoint = format!("/threads/{}/modify", thread_id);
        let body = if read {
            ModifyRequest {
                add_label_ids: vec![],
                remove_label_ids: vec!["UNREAD".to_string()],
            }
        } else {
            ModifyRequest {
                add_label_ids: vec!["UNREAD".to_string()],
                remove_label_ids: vec![],
            }
        };

        self.post_no_response(&endpoint, &body).await
    }

    async fn apply_label(&self, thread_id: &str, label: &str) -> Result<()> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let endpoint = format!("/threads/{}/modify", thread_id);
        let body = ModifyRequest {
            add_label_ids: vec![label.to_string()],
            remove_label_ids: vec![],
        };

        self.post_no_response(&endpoint, &body).await
    }

    async fn fetch_labels(&self) -> Result<Vec<Label>> {
        if !self.authenticated {
            return Err(ProviderError::Authentication(
                "not authenticated".to_string(),
            ));
        }

        let response: LabelsListResponse = self.get("/labels").await?;

        let labels = response
            .labels
            .unwrap_or_default()
            .into_iter()
            .filter(|l| {
                // Filter out system labels that shouldn't be shown
                l.label_type.as_deref() != Some("system")
                    || matches!(
                        l.id.as_str(),
                        "INBOX" | "STARRED" | "SENT" | "DRAFT" | "TRASH" | "SPAM"
                    )
            })
            .map(|l| {
                let is_system = l.label_type.as_deref() == Some("system");
                let provider_id = Some(l.id.clone());
                Label {
                    id: LabelId::from(l.id),
                    account_id: self.account_id.clone(),
                    name: l.name,
                    color: None, // Gmail API returns color separately, could be added
                    is_system,
                    provider_id,
                }
            })
            .collect();

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
                // Apply read status to all thread IDs
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
            PendingChangeType::RemoveLabel {
                thread_ids,
                label_id,
            } => {
                for thread_id in thread_ids {
                    let endpoint = format!("/threads/{}/modify", thread_id.0);
                    let body = ModifyRequest {
                        add_label_ids: vec![],
                        remove_label_ids: vec![label_id.0.clone()],
                    };
                    self.post_no_response(&endpoint, &body).await?;
                }
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

    #[test]
    fn gmail_provider_creation() {
        let provider = GmailProvider::new(AccountId::from("test-account"));
        assert_eq!(provider.account_id().0, "test-account");
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn gmail_provider_type() {
        let provider = GmailProvider::new(AccountId::from("test-account"));
        assert_eq!(provider.provider_type(), ProviderType::Gmail);
    }

    #[tokio::test]
    async fn gmail_provider_authenticate() {
        let mut provider = GmailProvider::new(AccountId::from("test-account"));
        assert!(!provider.is_authenticated());

        let result = provider.authenticate().await;
        assert!(result.is_ok());
        assert!(provider.is_authenticated());
    }

    #[tokio::test]
    async fn gmail_provider_requires_auth() {
        let provider = GmailProvider::new(AccountId::from("test-account"));

        let result = provider.fetch_threads("INBOX", Pagination::default()).await;
        assert!(matches!(result, Err(ProviderError::Authentication(_))));
    }

    #[tokio::test]
    async fn gmail_provider_fetch_threads_empty() {
        let mut provider = GmailProvider::new(AccountId::from("test-account"));
        provider.authenticate().await.unwrap();

        let result = provider
            .fetch_threads("INBOX", Pagination::with_limit(10))
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn gmail_provider_stub_operations() {
        let mut provider = GmailProvider::new(AccountId::from("test-account"));
        provider.authenticate().await.unwrap();

        // All stub operations should succeed (they're no-ops for now)
        assert!(provider.archive(&["thread-1".to_string()]).await.is_ok());
        assert!(provider.trash(&["thread-1".to_string()]).await.is_ok());
        assert!(provider.star("thread-1", true).await.is_ok());
        assert!(provider.mark_read("thread-1", true).await.is_ok());
        assert!(provider.apply_label("thread-1", "Work").await.is_ok());
        assert!(provider.fetch_labels().await.is_ok());
    }
}
