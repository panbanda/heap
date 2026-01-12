//! Embedding engine for semantic search.
//!
//! Uses Candle to run embedding models locally for privacy-preserving
//! semantic search over email content.

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::path::PathBuf;
use tokenizers::Tokenizer;

use crate::domain::{Email, EmailId};
use crate::embedding::VectorStore;

/// A vector embedding representing text semantics.
///
/// The embedding dimensionality depends on the model used
/// (e.g., 384 for MiniLM, 768 for BERT base).
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The embedding vector.
    pub values: Vec<f32>,
}

impl Embedding {
    /// Creates a new embedding from a vector of values.
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Returns the dimensionality of this embedding.
    pub fn dimension(&self) -> usize {
        self.values.len()
    }

    /// Computes cosine similarity with another embedding.
    ///
    /// Returns a value between -1.0 and 1.0, where 1.0 means identical.
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        if self.values.len() != other.values.len() {
            return 0.0;
        }

        let dot: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.values.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// L2 normalizes the embedding in place.
    pub fn normalize(&mut self) {
        let norm: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut self.values {
                *v /= norm;
            }
        }
    }
}

/// Configuration for the embedding engine.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Path to the model weights.
    pub model_path: Option<PathBuf>,
    /// Model identifier for downloading from Hugging Face.
    pub model_id: String,
    /// Maximum sequence length for tokenization.
    pub max_seq_length: usize,
    /// Whether to use GPU acceleration if available.
    pub use_gpu: bool,
    /// Whether to use fallback (hash-based) embeddings when model unavailable.
    pub use_fallback: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            model_id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            max_seq_length: 256,
            use_gpu: false,
            use_fallback: true,
        }
    }
}

/// Engine for generating text embeddings using local ML models.
///
/// The engine uses Candle for inference, avoiding external API calls
/// to maintain user privacy.
pub struct EmbeddingEngine {
    config: EmbeddingConfig,
    vector_store: VectorStore,
    model: Option<BertModel>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    initialized: bool,
}

impl EmbeddingEngine {
    /// Creates a new embedding engine with the given configuration.
    pub fn new(config: EmbeddingConfig, vector_store: VectorStore) -> Self {
        let device = if config.use_gpu {
            Device::cuda_if_available(0).unwrap_or(Device::Cpu)
        } else {
            Device::Cpu
        };

        Self {
            config,
            vector_store,
            model: None,
            tokenizer: None,
            device,
            initialized: false,
        }
    }

    /// Creates an embedding engine with default configuration.
    pub fn with_defaults(vector_store: VectorStore) -> Self {
        Self::new(EmbeddingConfig::default(), vector_store)
    }

    /// Initializes the model, loading weights from disk or downloading if needed.
    ///
    /// This should be called before using `embed()` or `index_email()`.
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        tracing::info!(
            model_id = %self.config.model_id,
            device = ?self.device,
            "Initializing embedding model"
        );

        match self.load_model() {
            Ok(()) => {
                self.initialized = true;
                tracing::info!("Embedding model loaded successfully");
            }
            Err(e) => {
                if self.config.use_fallback {
                    tracing::warn!(
                        error = %e,
                        "Failed to load embedding model, using fallback hash-based embeddings"
                    );
                    self.initialized = true;
                } else {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Loads the model and tokenizer from HuggingFace Hub.
    fn load_model(&mut self) -> Result<()> {
        let api = Api::new().context("Failed to create HuggingFace API client")?;
        let repo = api.repo(Repo::new(self.config.model_id.clone(), RepoType::Model));

        // Download model files
        let config_path = repo
            .get("config.json")
            .context("Failed to get config.json")?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .context("Failed to get tokenizer.json")?;
        let weights_path = repo
            .get("model.safetensors")
            .or_else(|_| repo.get("pytorch_model.bin"))
            .context("Failed to get model weights")?;

        // Load configuration
        let config_str =
            std::fs::read_to_string(&config_path).context("Failed to read config.json")?;
        let bert_config: BertConfig =
            serde_json::from_str(&config_str).context("Failed to parse config.json")?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        // Load model weights
        let vb = if weights_path
            .extension()
            .is_some_and(|ext| ext == "safetensors")
        {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &self.device)
                    .context("Failed to load safetensors weights")?
            }
        } else {
            VarBuilder::from_pth(weights_path, DType::F32, &self.device)
                .context("Failed to load PyTorch weights")?
        };

        // Build model
        let model = BertModel::load(vb, &bert_config).context("Failed to build BERT model")?;

        self.model = Some(model);
        self.tokenizer = Some(tokenizer);

        Ok(())
    }

    /// Generates an embedding for the given text.
    ///
    /// The text is tokenized, passed through the model, and the output
    /// is pooled to produce a fixed-size embedding vector.
    pub fn embed(&self, text: &str) -> Result<Embedding> {
        if let (Some(model), Some(tokenizer)) = (&self.model, &self.tokenizer) {
            self.embed_with_model(model, tokenizer, text)
        } else {
            // Fallback to hash-based pseudo-embeddings
            self.embed_fallback(text)
        }
    }

    /// Generates an embedding using the loaded model.
    fn embed_with_model(
        &self,
        model: &BertModel,
        tokenizer: &Tokenizer,
        text: &str,
    ) -> Result<Embedding> {
        // Tokenize the input
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let token_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        // Truncate to max sequence length
        let seq_len = token_ids.len().min(self.config.max_seq_length);
        let token_ids: Vec<u32> = token_ids[..seq_len].to_vec();
        let attention_mask: Vec<u32> = attention_mask[..seq_len].to_vec();

        // Create tensors
        let token_ids_tensor = Tensor::new(&token_ids[..], &self.device)?.unsqueeze(0)?;
        let attention_mask_tensor = Tensor::new(&attention_mask[..], &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::zeros_like(&token_ids_tensor)?;

        // Forward pass
        let output = model.forward(
            &token_ids_tensor,
            &token_type_ids,
            Some(&attention_mask_tensor),
        )?;

        // Mean pooling over sequence dimension (dim 1)
        let mask_expanded = attention_mask_tensor
            .unsqueeze(2)?
            .to_dtype(DType::F32)?
            .broadcast_as(output.shape())?;

        let sum_embeddings = (output * &mask_expanded)?.sum(1)?;
        let sum_mask = mask_expanded.sum(1)?.clamp(1e-9, f64::MAX)?;
        let mean_pooled = sum_embeddings.broadcast_div(&sum_mask)?;

        // Extract values
        let values: Vec<f32> = mean_pooled.squeeze(0)?.to_vec1()?;

        let mut embedding = Embedding::new(values);
        embedding.normalize();

        Ok(embedding)
    }

    /// Generates a fallback hash-based pseudo-embedding.
    fn embed_fallback(&self, text: &str) -> Result<Embedding> {
        let hash = Self::simple_hash(text);
        let dimension = 384; // MiniLM dimension
        let values: Vec<f32> = (0..dimension)
            .map(|i| {
                let seed = hash.wrapping_add(i as u64);
                (seed as f32 / u64::MAX as f32) * 2.0 - 1.0
            })
            .collect();

        let mut embedding = Embedding::new(values);
        embedding.normalize();
        Ok(embedding)
    }

    /// Searches for emails similar to the query embedding.
    ///
    /// Returns email IDs with their similarity scores, sorted by relevance.
    pub fn search(&self, query_embedding: &Embedding, limit: usize) -> Result<Vec<(EmailId, f32)>> {
        self.vector_store.search(query_embedding, limit)
    }

    /// Indexes an email by generating and storing its embedding.
    ///
    /// Combines subject and body text for a comprehensive representation.
    pub fn index_email(&mut self, email: &Email) -> Result<()> {
        let text = Self::email_to_text(email);
        let embedding = self.embed(&text)?;
        self.vector_store.insert(&email.id, embedding)?;
        Ok(())
    }

    /// Returns whether the model is loaded (vs using fallback).
    pub fn is_model_loaded(&self) -> bool {
        self.model.is_some() && self.tokenizer.is_some()
    }

    /// Returns the underlying vector store.
    pub fn vector_store(&self) -> &VectorStore {
        &self.vector_store
    }

    /// Returns a mutable reference to the underlying vector store.
    pub fn vector_store_mut(&mut self) -> &mut VectorStore {
        &mut self.vector_store
    }

    /// Converts an email to indexable text.
    fn email_to_text(email: &Email) -> String {
        let mut parts = Vec::new();

        if let Some(subject) = &email.subject {
            parts.push(subject.clone());
        }

        if let Some(body) = &email.body_text {
            parts.push(body.clone());
        }

        parts.join(" ")
    }

    /// Simple non-cryptographic hash for deterministic stub embeddings.
    fn simple_hash(text: &str) -> u64 {
        let mut hash: u64 = 5381;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AccountId, Address, MessageId, ThreadId};
    use chrono::Utc;

    fn make_test_email(id: &str, subject: &str, body: &str) -> Email {
        Email {
            id: EmailId::from(id),
            account_id: AccountId::from("test-account"),
            thread_id: ThreadId::from("thread-1"),
            message_id: MessageId::from("<test@example.com>"),
            in_reply_to: None,
            references: vec![],
            from: Address::new("sender@example.com"),
            to: vec![Address::new("recipient@example.com")],
            cc: vec![],
            bcc: vec![],
            subject: Some(subject.to_string()),
            body_text: Some(body.to_string()),
            body_html: None,
            snippet: body.chars().take(50).collect(),
            date: Utc::now(),
            is_read: false,
            is_starred: false,
            is_draft: false,
            labels: vec![],
            attachments: vec![],
        }
    }

    #[test]
    fn embedding_dimension() {
        let embedding = Embedding::new(vec![0.1, 0.2, 0.3]);
        assert_eq!(embedding.dimension(), 3);
    }

    #[test]
    fn cosine_similarity_identical() {
        let a = Embedding::new(vec![1.0, 0.0, 0.0]);
        let b = Embedding::new(vec![1.0, 0.0, 0.0]);
        let similarity = a.cosine_similarity(&b);
        assert!((similarity - 1.0).abs() < 0.0001);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = Embedding::new(vec![1.0, 0.0]);
        let b = Embedding::new(vec![0.0, 1.0]);
        let similarity = a.cosine_similarity(&b);
        assert!(similarity.abs() < 0.0001);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = Embedding::new(vec![1.0, 0.0]);
        let b = Embedding::new(vec![-1.0, 0.0]);
        let similarity = a.cosine_similarity(&b);
        assert!((similarity + 1.0).abs() < 0.0001);
    }

    #[test]
    fn cosine_similarity_mismatched_dims() {
        let a = Embedding::new(vec![1.0, 0.0]);
        let b = Embedding::new(vec![1.0, 0.0, 0.0]);
        let similarity = a.cosine_similarity(&b);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn embedding_normalize() {
        let mut embedding = Embedding::new(vec![3.0, 4.0]);
        embedding.normalize();
        // Norm should be 1.0
        let norm: f32 = embedding.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
        // Values should be 0.6, 0.8
        assert!((embedding.values[0] - 0.6).abs() < 0.0001);
        assert!((embedding.values[1] - 0.8).abs() < 0.0001);
    }

    #[test]
    fn embed_fallback_produces_consistent_output() {
        let store = VectorStore::new();
        let engine = EmbeddingEngine::with_defaults(store);

        let text = "Hello, world!";
        let emb1 = engine.embed(text).unwrap();
        let emb2 = engine.embed(text).unwrap();

        assert_eq!(emb1.values, emb2.values);
    }

    #[test]
    fn embed_fallback_different_texts_produce_different_embeddings() {
        let store = VectorStore::new();
        let engine = EmbeddingEngine::with_defaults(store);

        let emb1 = engine.embed("Hello").unwrap();
        let emb2 = engine.embed("Goodbye").unwrap();

        assert_ne!(emb1.values, emb2.values);
    }

    #[test]
    fn embed_fallback_produces_normalized_embeddings() {
        let store = VectorStore::new();
        let engine = EmbeddingEngine::with_defaults(store);

        let embedding = engine.embed("Test text").unwrap();
        let norm: f32 = embedding.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
    }

    #[test]
    fn index_and_search_email() {
        let store = VectorStore::new();
        let mut engine = EmbeddingEngine::with_defaults(store);

        let email = make_test_email("email-1", "Meeting tomorrow", "Let's discuss the project.");
        engine.index_email(&email).unwrap();

        let query = engine.embed("project meeting").unwrap();
        let results = engine.search(&query, 10).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, EmailId::from("email-1"));
    }

    #[test]
    fn email_to_text_combines_subject_and_body() {
        let email = make_test_email("email-1", "Subject here", "Body content here");
        let text = EmbeddingEngine::email_to_text(&email);
        assert!(text.contains("Subject here"));
        assert!(text.contains("Body content here"));
    }

    #[test]
    fn default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.model_id, "sentence-transformers/all-MiniLM-L6-v2");
        assert_eq!(config.max_seq_length, 256);
        assert!(!config.use_gpu);
        assert!(config.use_fallback);
    }

    #[test]
    fn engine_without_model_uses_fallback() {
        let store = VectorStore::new();
        let engine = EmbeddingEngine::with_defaults(store);
        assert!(!engine.is_model_loaded());

        // Should still work with fallback
        let embedding = engine.embed("test").unwrap();
        assert_eq!(embedding.dimension(), 384);
    }
}
