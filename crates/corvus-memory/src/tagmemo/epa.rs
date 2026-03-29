//! EPA (Embedding Projection Analysis) module
//!
//! Provides embedding projection analysis with logic depth calculation,
//! resonance detection, and semantic consistency verification.

use ndarray::Array1;
use serde::{Deserialize, Serialize};

/// Result of EPA analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpaAnalysis {
    /// Logic depth of the embedding
    pub logic_depth: f32,
    /// Resonance score (0.0-1.0)
    pub resonance: f32,
    /// Semantic consistency score
    pub consistency: f32,
    /// Dominant semantic dimensions
    pub dominant_dimensions: Vec<usize>,
    /// Energy distribution across dimensions
    pub energy_distribution: Vec<f32>,
}

/// EPA module for embedding analysis
pub struct EpaModule {
    /// Reference embedding vectors for comparison
    reference_vectors: Vec<Array1<f32>>,
    /// Dimension weights
    dimension_weights: Array1<f32>,
    /// Logic depth thresholds
    depth_thresholds: Vec<f32>,
}

impl EpaModule {
    /// Create a new EPA module
    pub fn new(embedding_dim: usize) -> Self {
        let dimension_weights = Array1::ones(embedding_dim);
        let depth_thresholds = vec![0.1, 0.3, 0.5, 0.7, 0.9];

        Self {
            reference_vectors: Vec::new(),
            dimension_weights,
            depth_thresholds,
        }
    }

    /// Add a reference vector
    pub fn add_reference(&mut self, vector: Array1<f32>) {
        self.reference_vectors.push(vector);
    }

    /// Analyze an embedding vector
    pub fn analyze(&self, embedding: &Array1<f32>) -> EpaAnalysis {
        let normalized = self.normalize(embedding);
        let energy_distribution = self.compute_energy_distribution(&normalized);
        let logic_depth = self.compute_logic_depth(&energy_distribution);
        let resonance = self.compute_resonance(&normalized);
        let consistency = self.compute_consistency(&energy_distribution);
        let dominant_dimensions = self.find_dominant_dimensions(&energy_distribution);

        EpaAnalysis {
            logic_depth,
            resonance,
            consistency,
            dominant_dimensions,
            energy_distribution,
        }
    }

    /// Normalize embedding vector
    fn normalize(&self, embedding: &Array1<f32>) -> Array1<f32> {
        let norm = embedding.dot(embedding).sqrt();
        if norm > 0.0 {
            embedding / norm
        } else {
            embedding.clone()
        }
    }

    /// Compute energy distribution across dimensions
    fn compute_energy_distribution(&self, normalized: &Array1<f32>) -> Vec<f32> {
        let squared = normalized.mapv(|x| x * x) * &self.dimension_weights;
        let total_energy = squared.sum();

        if total_energy > 0.0 {
            squared.iter().map(|&x| x / total_energy).collect()
        } else {
            vec![0.0; normalized.len()]
        }
    }

    /// Compute logic depth based on energy distribution
    fn compute_logic_depth(&self, energy_distribution: &[f32]) -> f32 {
        let mut sorted_energy = energy_distribution.to_vec();
        sorted_energy.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let mut cumulative = 0.0;
        let mut depth_level = 0;

        for &energy in &sorted_energy {
            cumulative += energy;
            // Find which threshold we've crossed
            while depth_level < self.depth_thresholds.len()
                && cumulative >= self.depth_thresholds[depth_level]
            {
                depth_level += 1;
            }
        }

        // Normalize to 0.0-1.0
        depth_level as f32 / self.depth_thresholds.len() as f32
    }

    /// Compute resonance with reference vectors
    fn compute_resonance(&self, normalized: &Array1<f32>) -> f32 {
        if self.reference_vectors.is_empty() {
            return 0.5;
        }

        let mut max_similarity: f32 = 0.0;

        for reference in &self.reference_vectors {
            let ref_normalized = self.normalize(reference);
            let similarity = normalized.dot(&ref_normalized).max(0.0);
            max_similarity = max_similarity.max(similarity);
        }

        max_similarity
    }

    /// Compute semantic consistency
    fn compute_consistency(&self, energy_distribution: &[f32]) -> f32 {
        let entropy = -energy_distribution.iter()
            .filter(|&&x| x > 0.0)
            .map(|&x| x * x.log2())
            .sum::<f32>();

        let max_entropy = (energy_distribution.len() as f32).log2();
        if max_entropy > 0.0 {
            1.0 - (entropy / max_entropy)
        } else {
            1.0
        }
    }

    /// Find dominant semantic dimensions
    fn find_dominant_dimensions(&self, energy_distribution: &[f32]) -> Vec<usize> {
        let mut dim_indices: Vec<usize> = (0..energy_distribution.len()).collect();
        dim_indices.sort_by(|&a, &b| {
            energy_distribution[b].partial_cmp(&energy_distribution[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let threshold = 0.05;
        dim_indices.into_iter()
            .filter(|&i| energy_distribution[i] > threshold)
            .take(10)
            .collect()
    }

    /// Compare two embeddings for similarity
    pub fn compare(&self, a: &Array1<f32>, b: &Array1<f32>) -> f32 {
        let a_norm = self.normalize(a);
        let b_norm = self.normalize(b);
        a_norm.dot(&b_norm).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr1;

    #[test]
    fn test_epa_creation() {
        let epa = EpaModule::new(128);
        assert_eq!(epa.dimension_weights.len(), 128);
    }

    #[test]
    fn test_epa_analysis() {
        let epa = EpaModule::new(10);
        let embedding = arr1(&[1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        let analysis = epa.analyze(&embedding);
        assert!(analysis.logic_depth >= 0.0 && analysis.logic_depth <= 1.0);
        assert!(analysis.resonance >= 0.0 && analysis.resonance <= 1.0);
        assert!(analysis.consistency >= 0.0 && analysis.consistency <= 1.0);
    }
}
