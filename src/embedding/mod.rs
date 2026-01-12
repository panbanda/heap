//! Vector embedding and semantic search.
//!
//! This module provides local ML-based text embeddings using Candle
//! for privacy-preserving semantic search over email content.
//!
//! # Architecture
//!
//! - [`EmbeddingEngine`] - Generates embeddings using local transformer models
//! - [`VectorStore`] - Stores and searches embeddings by similarity
//! - [`Embedding`] - A vector representation of text semantics
//!
//! # Example
//!
//! ```ignore
//! use margin::embedding::{EmbeddingEngine, VectorStore};
//!
//! let store = VectorStore::new();
//! let mut engine = EmbeddingEngine::with_defaults(store);
//!
//! // Index an email
//! engine.index_email(&email)?;
//!
//! // Search for similar content
//! let query = engine.embed("project deadline")?;
//! let results = engine.search(&query, 10)?;
//! ```

mod engine;
mod models;
mod vector_store;

pub use engine::{Embedding, EmbeddingConfig, EmbeddingEngine};
pub use models::{DownloadStatus, ModelInfo, ModelRegistry, ModelType};
pub use vector_store::VectorStore;
