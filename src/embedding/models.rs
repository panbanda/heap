//! Embedding model definitions and configuration.
//!
//! This module defines the available embedding models and their configurations
//! for generating text embeddings used in semantic search.

use serde::{Deserialize, Serialize};

/// Available embedding model types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// MiniLM model - fast, small, good for general text.
    MiniLm,
    /// All-MiniLM-L6-v2 - balanced speed and quality.
    #[default]
    AllMiniLmL6V2,
    /// BGE-Small - optimized for retrieval tasks.
    BgeSmall,
    /// E5-Small - good for asymmetric search.
    E5Small,
}

impl ModelType {
    /// Returns the Hugging Face model ID.
    pub fn hf_model_id(&self) -> &'static str {
        match self {
            Self::MiniLm => "sentence-transformers/paraphrase-MiniLM-L6-v2",
            Self::AllMiniLmL6V2 => "sentence-transformers/all-MiniLM-L6-v2",
            Self::BgeSmall => "BAAI/bge-small-en-v1.5",
            Self::E5Small => "intfloat/e5-small-v2",
        }
    }

    /// Returns the expected embedding dimension.
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::MiniLm => 384,
            Self::AllMiniLmL6V2 => 384,
            Self::BgeSmall => 384,
            Self::E5Small => 384,
        }
    }

    /// Returns the maximum sequence length.
    pub fn max_seq_length(&self) -> usize {
        match self {
            Self::MiniLm => 256,
            Self::AllMiniLmL6V2 => 256,
            Self::BgeSmall => 512,
            Self::E5Small => 512,
        }
    }

    /// Returns whether this model requires a query prefix.
    pub fn requires_query_prefix(&self) -> bool {
        matches!(self, Self::E5Small)
    }

    /// Returns the query prefix if required.
    pub fn query_prefix(&self) -> Option<&'static str> {
        match self {
            Self::E5Small => Some("query: "),
            _ => None,
        }
    }

    /// Returns the document prefix if required.
    pub fn document_prefix(&self) -> Option<&'static str> {
        match self {
            Self::E5Small => Some("passage: "),
            _ => None,
        }
    }
}

/// Model download status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    /// Model is not downloaded.
    NotDownloaded,
    /// Model is being downloaded.
    Downloading,
    /// Model is downloaded and ready.
    Ready,
    /// Download failed.
    Failed,
}

/// Information about a model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model type.
    pub model_type: ModelType,
    /// Human-readable name.
    pub name: String,
    /// Description of the model.
    pub description: String,
    /// Approximate size in bytes.
    pub size_bytes: u64,
    /// Download status.
    pub status: DownloadStatus,
}

impl ModelInfo {
    /// Creates info for a model type.
    pub fn for_model(model_type: ModelType) -> Self {
        let (name, description, size_bytes) = match model_type {
            ModelType::MiniLm => (
                "MiniLM",
                "Fast, small model for general text embedding",
                90_000_000,
            ),
            ModelType::AllMiniLmL6V2 => (
                "All-MiniLM-L6-v2",
                "Balanced model with good quality and speed",
                90_000_000,
            ),
            ModelType::BgeSmall => (
                "BGE-Small",
                "Optimized for retrieval and semantic search",
                130_000_000,
            ),
            ModelType::E5Small => (
                "E5-Small",
                "Good for asymmetric search (query vs document)",
                130_000_000,
            ),
        };

        Self {
            model_type,
            name: name.to_string(),
            description: description.to_string(),
            size_bytes,
            status: DownloadStatus::NotDownloaded,
        }
    }

    /// Returns the size as a human-readable string.
    pub fn size_human(&self) -> String {
        let mb = self.size_bytes as f64 / 1_000_000.0;
        format!("{:.1} MB", mb)
    }
}

/// Model registry for managing available models.
#[derive(Debug, Default)]
pub struct ModelRegistry {
    models: Vec<ModelInfo>,
}

impl ModelRegistry {
    /// Creates a new registry with all available models.
    pub fn new() -> Self {
        let models = vec![
            ModelInfo::for_model(ModelType::MiniLm),
            ModelInfo::for_model(ModelType::AllMiniLmL6V2),
            ModelInfo::for_model(ModelType::BgeSmall),
            ModelInfo::for_model(ModelType::E5Small),
        ];
        Self { models }
    }

    /// Gets all available models.
    pub fn all_models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Gets info for a specific model type.
    pub fn get_model(&self, model_type: ModelType) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.model_type == model_type)
    }

    /// Gets info for a specific model type mutably.
    pub fn get_model_mut(&mut self, model_type: ModelType) -> Option<&mut ModelInfo> {
        self.models.iter_mut().find(|m| m.model_type == model_type)
    }

    /// Updates the status of a model.
    pub fn set_status(&mut self, model_type: ModelType, status: DownloadStatus) {
        if let Some(model) = self.get_model_mut(model_type) {
            model.status = status;
        }
    }

    /// Gets all ready models.
    pub fn ready_models(&self) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.status == DownloadStatus::Ready)
            .collect()
    }

    /// Gets the default model.
    pub fn default_model(&self) -> &ModelInfo {
        self.get_model(ModelType::default())
            .expect("default model should always exist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_type_properties() {
        let model = ModelType::AllMiniLmL6V2;
        assert_eq!(model.embedding_dim(), 384);
        assert_eq!(model.max_seq_length(), 256);
        assert!(!model.requires_query_prefix());
    }

    #[test]
    fn e5_model_prefix() {
        let model = ModelType::E5Small;
        assert!(model.requires_query_prefix());
        assert_eq!(model.query_prefix(), Some("query: "));
        assert_eq!(model.document_prefix(), Some("passage: "));
    }

    #[test]
    fn model_info() {
        let info = ModelInfo::for_model(ModelType::AllMiniLmL6V2);
        assert_eq!(info.name, "All-MiniLM-L6-v2");
        assert!(info.size_human().contains("MB"));
    }

    #[test]
    fn model_registry() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.all_models().len(), 4);

        let default = registry.default_model();
        assert_eq!(default.model_type, ModelType::AllMiniLmL6V2);
    }

    #[test]
    fn registry_status_update() {
        let mut registry = ModelRegistry::new();

        registry.set_status(ModelType::MiniLm, DownloadStatus::Ready);

        let model = registry.get_model(ModelType::MiniLm).unwrap();
        assert_eq!(model.status, DownloadStatus::Ready);

        let ready = registry.ready_models();
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn model_serialization() {
        let model = ModelType::BgeSmall;
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(json, "\"bge_small\"");

        let deserialized: ModelType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ModelType::BgeSmall);
    }
}
