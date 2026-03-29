//! OpenAI embedding model implementation

use crate::openai::client::OpenAIClient;
use crate::openai::types::*;
use corvus_core::embedding::*;
use corvus_core::error::{EmbeddingError, Result};
use std::sync::Arc;

/// OpenAI embedding model
pub struct OpenAIEmbeddingModel {
    client: Arc<OpenAIClient>,
    model: String,
    dimensions: Option<usize>,
}

impl OpenAIEmbeddingModel {
    /// Create a new OpenAI embedding model
    pub fn new(client: Arc<OpenAIClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            dimensions: None,
        }
    }

    /// Create with text-embedding-3-small
    pub fn small(client: Arc<OpenAIClient>) -> Self {
        Self::new(client, crate::openai::models::TEXT_EMBEDDING_3_SMALL)
    }

    /// Create with text-embedding-3-large
    pub fn large(client: Arc<OpenAIClient>) -> Self {
        Self::new(client, crate::openai::models::TEXT_EMBEDDING_3_LARGE)
    }

    /// Create with text-embedding-ada-002
    pub fn ada_002(client: Arc<OpenAIClient>) -> Self {
        Self::new(client, crate::openai::models::TEXT_EMBEDDING_ADA_002)
    }

    /// Set custom dimensions (only for v3 models)
    pub fn with_dimensions(mut self, dims: usize) -> Self {
        self.dimensions = Some(dims);
        self
    }
}

#[async_trait::async_trait]
impl EmbeddingModel for OpenAIEmbeddingModel {
    fn ndims(&self) -> usize {
        match (self.model.as_str(), self.dimensions) {
            ("text-embedding-3-small", Some(d)) => d,
            ("text-embedding-3-large", Some(d)) => d,
            ("text-embedding-3-small", None) => 1536,
            ("text-embedding-3-large", None) => 3072,
            ("text-embedding-ada-002", _) => 1536,
            _ => 1536,
        }
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    const MAX_DOCUMENTS: usize = 2048;

    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        if texts.len() > Self::MAX_DOCUMENTS {
            return Err(EmbeddingError::TooManyDocuments {
                max: Self::MAX_DOCUMENTS,
                got: texts.len(),
            }
            .into());
        }

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
            dimensions: self.dimensions,
        };

        let response = self.client.embeddings(request).await
            .map_err(|e| EmbeddingError::ApiRequest(e.to_string()))?;

        // Sort by index to ensure correct order
        let mut embeddings = response.data;
        embeddings.sort_by_key(|e| e.index);

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }
}
