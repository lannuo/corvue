//! Vector index for semantic search
//!
//! Provides a simple k-NN (k-nearest neighbors) index using cosine similarity
//! for efficient similarity search on embeddings.

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A vector entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Unique ID for this vector
    pub id: String,
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

/// A simple in-memory k-NN index
pub struct KnnIndex {
    /// Stored vectors
    entries: Vec<VectorEntry>,
    /// Normalized vectors for faster cosine similarity
    normalized_vectors: Option<Array2<f32>>,
    /// Whether the index needs to be rebuilt
    dirty: bool,
}

impl KnnIndex {
    /// Create a new empty k-NN index
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            normalized_vectors: None,
            dirty: false,
        }
    }

    /// Add a vector to the index
    pub fn add(&mut self, id: String, vector: Vec<f32>, metadata: HashMap<String, String>) {
        self.entries.push(VectorEntry { id, vector, metadata });
        self.dirty = true;
    }

    /// Remove a vector from the index by ID
    pub fn remove(&mut self, id: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.id != id);
        if self.entries.len() < initial_len {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Get a vector by ID
    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Build the index (compute normalized vectors)
    pub fn build(&mut self) {
        if !self.dirty && self.normalized_vectors.is_some() {
            return;
        }

        if self.entries.is_empty() {
            self.normalized_vectors = None;
            self.dirty = false;
            return;
        }

        let dim = self.entries[0].vector.len();
        let mut normalized = Array2::zeros((self.entries.len(), dim));

        for (i, entry) in self.entries.iter().enumerate() {
            let vec = Array1::from(entry.vector.clone());
            let norm = vec.dot(&vec).sqrt();
            if norm > 0.0 {
                normalized.row_mut(i).assign(&(vec / norm));
            }
        }

        self.normalized_vectors = Some(normalized);
        self.dirty = false;
    }

    /// Compute cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let a_arr = Array1::from(a.to_vec());
        let b_arr = Array1::from(b.to_vec());

        let dot = a_arr.dot(&b_arr);
        let norm_a = a_arr.dot(&a_arr).sqrt();
        let norm_b = b_arr.dot(&b_arr).sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Search for the k most similar vectors
    pub fn search(&mut self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() || k == 0 {
            return Vec::new();
        }

        self.build();

        // Normalize query vector
        let query_arr = Array1::from(query.to_vec());
        let query_norm = query_arr.dot(&query_arr).sqrt();
        let query_normalized = if query_norm > 0.0 {
            query_arr / query_norm
        } else {
            query_arr
        };

        // Compute similarities
        let mut scores = Vec::new();

        if let Some(norm_vecs) = &self.normalized_vectors {
            for (i, entry) in self.entries.iter().enumerate() {
                let sim = norm_vecs.row(i).dot(&query_normalized);
                scores.push((entry.id.clone(), sim));
            }
        } else {
            // Fallback to non-normalized
            for entry in &self.entries {
                let sim = Self::cosine_similarity(query, &entry.vector);
                scores.push((entry.id.clone(), sim));
            }
        }

        // Sort by similarity descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        scores.truncate(k);
        scores
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entry IDs
    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.id.as_str()).collect()
    }
}

impl Default for KnnIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_creation() {
        let index = KnnIndex::new();
        assert!(index.is_empty());
    }

    #[test]
    fn test_add_and_search() {
        let mut index = KnnIndex::new();

        // Add some test vectors
        index.add("vec1".to_string(), vec![1.0, 0.0, 0.0], HashMap::new());
        index.add("vec2".to_string(), vec![0.0, 1.0, 0.0], HashMap::new());
        index.add("vec3".to_string(), vec![0.0, 0.0, 1.0], HashMap::new());

        assert_eq!(index.len(), 3);

        // Search for similar to vec1
        let results = index.search(&vec![1.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "vec1");
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((KnnIndex::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(KnnIndex::cosine_similarity(&a, &c).abs() < 0.001);
    }
}
