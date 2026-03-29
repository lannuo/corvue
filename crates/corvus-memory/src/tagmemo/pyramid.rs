//! Residual Pyramid module
//!
//! Implements Gram-Schmidt orthogonal decomposition for semantic
//! energy analysis with multi-scale residual processing.

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single level in the residual pyramid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidLevel {
    /// Level index (0 = finest, n = coarsest)
    pub level: usize,
    /// Orthogonal basis vector for this level
    pub basis: Vec<f32>,
    /// Projection coefficient
    pub coefficient: f32,
    /// Residual energy at this level
    pub residual_energy: f32,
    /// Semantic tags associated with this level
    pub tags: Vec<String>,
}

/// Residual Pyramid for multi-scale semantic analysis
pub struct ResidualPyramid {
    /// Number of pyramid levels
    num_levels: usize,
    /// Embedding dimension
    embedding_dim: usize,
    /// Pyramid levels
    levels: Vec<PyramidLevel>,
    /// Orthogonal basis matrix
    basis_matrix: Option<Array2<f32>>,
    /// Energy history for tracking
    energy_history: VecDeque<Vec<f32>>,
    /// Maximum history size
    max_history: usize,
}

impl ResidualPyramid {
    /// Create a new residual pyramid
    pub fn new(num_levels: usize, embedding_dim: usize) -> Self {
        Self {
            num_levels,
            embedding_dim,
            levels: Vec::new(),
            basis_matrix: None,
            energy_history: VecDeque::new(),
            max_history: 100,
        }
    }

    /// Decompose an embedding using Gram-Schmidt orthogonalization
    pub fn decompose(&mut self, embedding: &Array1<f32>) -> anyhow::Result<&[PyramidLevel]> {
        let mut residual = embedding.clone();
        let mut basis_vectors: Vec<Array1<f32>> = Vec::new();
        let mut new_levels: Vec<PyramidLevel> = Vec::new();

        for level in 0..self.num_levels {
            let basis = if level < basis_vectors.len() {
                basis_vectors[level].clone()
            } else {
                self.create_basis_vector(&residual, level)
            };

            let orthogonal_basis = self.gram_schmidt_orthogonalize(&basis, &basis_vectors);
            let norm = orthogonal_basis.dot(&orthogonal_basis).sqrt();
            let normalized_basis = if norm > 1e-10 {
                orthogonal_basis / norm
            } else {
                self.standard_basis_vector(basis_vectors.len())
            };

            let coefficient = residual.dot(&normalized_basis);
            residual -= &(&normalized_basis * coefficient);
            let residual_energy = residual.dot(&residual);

            if level >= basis_vectors.len() {
                basis_vectors.push(normalized_basis.clone());
            }

            new_levels.push(PyramidLevel {
                level,
                basis: normalized_basis.to_vec(),
                coefficient,
                residual_energy,
                tags: Vec::new(),
            });
        }

        self.update_basis_matrix(&basis_vectors);
        self.levels = new_levels;
        self.track_energy();

        Ok(&self.levels)
    }

    /// Reconstruct embedding from pyramid levels
    pub fn reconstruct(&self, levels_to_use: Option<usize>) -> Array1<f32> {
        let num_levels = levels_to_use.unwrap_or(self.levels.len());
        let mut reconstructed = Array1::zeros(self.embedding_dim);

        for level in self.levels.iter().take(num_levels) {
            let basis = Array1::from(level.basis.clone());
            reconstructed += &(basis * level.coefficient);
        }

        reconstructed
    }

    /// Get semantic energy distribution across levels
    pub fn energy_distribution(&self) -> Vec<f32> {
        self.levels.iter()
            .map(|level| level.residual_energy)
            .collect()
    }

    /// Get the most significant level (highest coefficient magnitude)
    pub fn most_significant_level(&self) -> Option<&PyramidLevel> {
        self.levels.iter()
            .max_by(|a, b| a.coefficient.abs().partial_cmp(&b.coefficient.abs()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Add tags to a specific level
    pub fn add_tags_to_level(&mut self, level: usize, tags: Vec<String>) {
        if let Some(pyramid_level) = self.levels.get_mut(level) {
            pyramid_level.tags.extend(tags);
        }
    }

    /// Get tags across all levels
    pub fn all_tags(&self) -> Vec<&str> {
        self.levels.iter()
            .flat_map(|level| level.tags.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Compare two pyramids for similarity
    pub fn compare(&self, other: &ResidualPyramid) -> f32 {
        if self.levels.len() != other.levels.len() {
            return 0.0;
        }

        let mut similarity = 0.0;
        let mut weight_sum = 0.0;

        for (level_idx, (a, b)) in self.levels.iter().zip(other.levels.iter()).enumerate() {
            let weight = 1.0 / ((level_idx + 1) as f32);
            let basis_a = Array1::from(a.basis.clone());
            let basis_b = Array1::from(b.basis.clone());
            let basis_sim = basis_a.dot(&basis_b).abs();
            let coeff_sim = 1.0 - (a.coefficient - b.coefficient).abs() / (a.coefficient.abs().max(b.coefficient.abs()).max(1.0));

            similarity += (basis_sim * 0.7 + coeff_sim * 0.3) * weight;
            weight_sum += weight;
        }

        if weight_sum > 0.0 {
            similarity / weight_sum
        } else {
            0.0
        }
    }

    /// Get all pyramid levels
    pub fn levels(&self) -> &[PyramidLevel] {
        &self.levels
    }

    /// Get number of levels
    pub fn num_levels(&self) -> usize {
        self.num_levels
    }

    // Helper methods

    fn create_basis_vector(&self, residual: &Array1<f32>, level: usize) -> Array1<f32> {
        let residual_norm = residual.dot(residual).sqrt();
        if residual_norm > 1e-10 {
            residual.clone()
        } else {
            self.standard_basis_vector(level)
        }
    }

    fn standard_basis_vector(&self, index: usize) -> Array1<f32> {
        let mut v = Array1::zeros(self.embedding_dim);
        let idx = index % self.embedding_dim;
        v[idx] = 1.0;
        v
    }

    fn gram_schmidt_orthogonalize(&self, vector: &Array1<f32>, bases: &[Array1<f32>]) -> Array1<f32> {
        let mut result = vector.clone();

        for basis in bases {
            let projection = result.dot(basis);
            result -= &(basis * projection);
        }

        result
    }

    fn update_basis_matrix(&mut self, bases: &[Array1<f32>]) {
        if bases.is_empty() {
            self.basis_matrix = None;
            return;
        }

        let mut matrix = Array2::zeros((bases.len(), self.embedding_dim));
        for (i, basis) in bases.iter().enumerate() {
            matrix.row_mut(i).assign(basis);
        }

        self.basis_matrix = Some(matrix);
    }

    fn track_energy(&mut self) {
        let energies: Vec<f32> = self.levels.iter()
            .map(|level| level.residual_energy)
            .collect();

        self.energy_history.push_back(energies);

        if self.energy_history.len() > self.max_history {
            self.energy_history.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr1;

    #[test]
    fn test_pyramid_creation() {
        let pyramid = ResidualPyramid::new(5, 128);
        assert_eq!(pyramid.num_levels(), 5);
    }

    #[test]
    fn test_pyramid_decompose() -> anyhow::Result<()> {
        let mut pyramid = ResidualPyramid::new(3, 10);
        let embedding = arr1(&[1.0, 0.5, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        let levels = pyramid.decompose(&embedding)?;
        assert_eq!(levels.len(), 3);

        Ok(())
    }
}
