//! Embedding model trait and types

use crate::error::{EmbeddingError, Result};

/// An embedding vector
pub type Embedding = Vec<f32>;

/// Trait for embedding similarity calculations
pub trait EmbeddingSimilarity {
    /// Calculate cosine similarity with another embedding
    fn cosine_similarity(&self, other: &Self) -> f32;

    /// Calculate dot product with another embedding
    fn dot_product(&self, other: &Self) -> f32;

    /// Calculate the Euclidean norm (L2 norm)
    fn norm(&self) -> f32;
}

impl EmbeddingSimilarity for Embedding {
    fn cosine_similarity(&self, other: &Self) -> f32 {
        let dot = self.dot_product(other);
        let norm1 = self.norm();
        let norm2 = other.norm();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        dot / (norm1 * norm2)
    }

    fn dot_product(&self, other: &Self) -> f32 {
        self.iter().zip(other.iter()).map(|(a, b)| a * b).sum()
    }

    fn norm(&self) -> f32 {
        self.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
}

/// Trait for embedding models
#[async_trait::async_trait]
pub trait EmbeddingModel: Send + Sync {
    /// Get the number of dimensions
    fn ndims(&self) -> usize;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Maximum number of texts per batch
    const MAX_DOCUMENTS: usize = 1024;

    /// Embed multiple texts
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Embedding>>;

    /// Embed a single query text
    async fn embed_query(&self, query: &str) -> Result<Embedding> {
        let embeddings = self.embed_texts(&[query.to_string()]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embedding returned".to_string()).into())
    }
}
