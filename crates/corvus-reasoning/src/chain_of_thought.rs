//! Chain-of-Thought (CoT) reasoning
//!
//! Provides structured thinking steps, self-verification, and alternative path exploration.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Type of thought step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThoughtStepType {
    /// Initial observation or problem understanding
    Observation,
    /// Reasoning about the problem
    Reasoning,
    /// Planning an action
    Planning,
    /// Executing an action
    Execution,
    /// Verifying a result
    Verification,
    /// Correcting a mistake
    Correction,
    /// Drawing a conclusion
    Conclusion,
}

/// A single thought step in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    /// Unique step ID
    pub id: String,
    /// Type of thought step
    pub step_type: ThoughtStepType,
    /// The thought content
    pub content: String,
    /// Timestamp when this step was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Confidence in this step (0.0-1.0)
    pub confidence: f32,
    /// Optional parent step ID (for branching)
    pub parent_id: Option<String>,
    /// Optional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Result of verifying a thought or conclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the verification passed
    pub passed: bool,
    /// Verification details
    pub details: String,
    /// Suggested corrections if failed
    pub suggested_corrections: Vec<String>,
    /// Confidence in the verification (0.0-1.0)
    pub confidence: f32,
}

/// An alternative reasoning path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativePath {
    /// Path ID
    pub id: String,
    /// Description of this alternative approach
    pub description: String,
    /// The thought steps in this path
    pub steps: Vec<ThoughtStep>,
    /// Why this alternative was considered
    pub rationale: String,
    /// Score comparing to main path (0.0-1.0)
    pub score: f32,
}

/// Chain-of-Thought reasoning engine
pub struct ChainOfThought {
    /// The main chain of thoughts
    chain: Vec<ThoughtStep>,
    /// Alternative reasoning paths
    alternatives: Vec<AlternativePath>,
    /// Current active path ID (None = main path)
    active_path: Option<String>,
    /// Step history for rollback
    history: VecDeque<ThoughtStep>,
    /// Maximum history size
    max_history: usize,
}

impl ChainOfThought {
    /// Create a new Chain-of-Thought engine
    pub fn new() -> Self {
        Self {
            chain: Vec::new(),
            alternatives: Vec::new(),
            active_path: None,
            history: VecDeque::new(),
            max_history: 20,
        }
    }

    /// Create with custom configuration
    pub fn with_config(_max_steps: usize, _auto_verify: bool, _exploration_factor: f32) -> Self {
        Self {
            chain: Vec::new(),
            alternatives: Vec::new(),
            active_path: None,
            history: VecDeque::new(),
            max_history: 20,
        }
    }

    /// Generate a unique step ID
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("step_{:x}", nanos)
    }

    /// Add a thought step to the chain
    pub fn add_step(
        &mut self,
        step_type: ThoughtStepType,
        content: String,
        confidence: f32,
    ) -> &ThoughtStep {
        let parent_id = self.current_step().map(|s| s.id.clone());

        let step = ThoughtStep {
            id: Self::generate_id(),
            step_type,
            content,
            timestamp: chrono::Utc::now(),
            confidence: confidence.clamp(0.0, 1.0),
            parent_id,
            metadata: std::collections::HashMap::new(),
        };

        // Save to history before modifying
        if let Some(last) = self.chain.last() {
            self.history.push_back(last.clone());
            if self.history.len() > self.max_history {
                self.history.pop_front();
            }
        }

        self.chain.push(step.clone());

        self.chain.last().unwrap()
    }

    /// Add an observation step
    pub fn observe(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Observation, content, confidence)
    }

    /// Add a reasoning step
    pub fn reason(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Reasoning, content, confidence)
    }

    /// Add a planning step
    pub fn plan(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Planning, content, confidence)
    }

    /// Add an execution step
    pub fn execute(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Execution, content, confidence)
    }

    /// Add a verification step
    pub fn verify(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Verification, content, confidence)
    }

    /// Add a correction step
    pub fn correct(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Correction, content, confidence)
    }

    /// Add a conclusion step
    pub fn conclude(&mut self, content: String, confidence: f32) -> &ThoughtStep {
        self.add_step(ThoughtStepType::Conclusion, content, confidence)
    }

    /// Get the current (last) step
    pub fn current_step(&self) -> Option<&ThoughtStep> {
        self.chain.last()
    }

    /// Get all steps in the chain
    pub fn steps(&self) -> &[ThoughtStep] {
        &self.chain
    }

    /// Get steps by type
    pub fn steps_by_type(&self, step_type: ThoughtStepType) -> Vec<&ThoughtStep> {
        self.chain
            .iter()
            .filter(|s| s.step_type == step_type)
            .collect()
    }

    /// Rollback to a previous step
    pub fn rollback(&mut self, steps: usize) -> bool {
        if steps == 0 || self.chain.len() <= steps {
            return false;
        }

        for _ in 0..steps {
            if let Some(step) = self.chain.pop() {
                self.history.push_back(step);
            }
        }

        true
    }

    /// Undo the last rollback
    pub fn undo_rollback(&mut self, steps: usize) -> bool {
        if steps == 0 || self.history.is_empty() {
            return false;
        }

        let steps_to_restore = steps.min(self.history.len());
        for _ in 0..steps_to_restore {
            if let Some(step) = self.history.pop_back() {
                self.chain.push(step);
            }
        }

        true
    }

    /// Create an alternative reasoning path
    pub fn create_alternative(
        &mut self,
        from_step_id: &str,
        description: String,
        rationale: String,
    ) -> Option<&AlternativePath> {
        // Find the branch point
        let branch_index = self
            .chain
            .iter()
            .position(|s| s.id == from_step_id)?;

        let path_id = format!("alt_{}", Self::generate_id());
        let steps = self.chain[0..=branch_index].to_vec();

        let alternative = AlternativePath {
            id: path_id.clone(),
            description,
            steps,
            rationale,
            score: 0.5,
        };

        self.alternatives.push(alternative);
        self.alternatives.last()
    }

    /// Switch to an alternative path
    pub fn switch_to_alternative(&mut self, path_id: &str) -> bool {
        if path_id == "main" {
            self.active_path = None;
            return true;
        }

        if let Some(_alt) = self.alternatives.iter().find(|a| a.id == path_id) {
            self.active_path = Some(path_id.to_string());
            return true;
        }

        false
    }

    /// Get all alternative paths
    pub fn alternatives(&self) -> &[AlternativePath] {
        &self.alternatives
    }

    /// Get the best alternative with the highest score
    pub fn best_alternative(&self) -> Option<&AlternativePath> {
        self.alternatives
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Calculate overall confidence in the current chain
    pub fn overall_confidence(&self) -> f32 {
        if self.chain.is_empty() {
            return 0.0;
        }

        // Weight later steps more heavily
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        for (i, step) in self.chain.iter().enumerate() {
            let weight = (i + 1) as f32;
            weighted_sum += step.confidence * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Verify the entire chain for consistency
    pub fn verify_chain(&self) -> VerificationResult {
        if self.chain.is_empty() {
            return VerificationResult {
                passed: true,
                details: "Empty chain".to_string(),
                suggested_corrections: Vec::new(),
                confidence: 1.0,
            };
        }

        let mut issues = Vec::new();
        let mut corrections = Vec::new();

        // Check for conclusion step
        let has_conclusion = self
            .chain
            .iter()
            .any(|s| s.step_type == ThoughtStepType::Conclusion);

        if !has_conclusion {
            issues.push("No conclusion step found".to_string());
            corrections.push("Add a conclusion step".to_string());
        }

        // Check confidence trends
        let confidence_trend = self.chain.windows(2).any(|w| w[1].confidence < w[0].confidence - 0.3);

        if confidence_trend {
            issues.push("Confidence dropped significantly".to_string());
            corrections.push("Review steps with confidence drops".to_string());
        }

        VerificationResult {
            passed: issues.is_empty(),
            details: if issues.is_empty() {
                "Chain verification passed".to_string()
            } else {
                issues.join("; ")
            },
            suggested_corrections: corrections,
            confidence: if issues.is_empty() { 0.9 } else { 0.5 },
        }
    }

    /// Clear the chain and start fresh
    pub fn clear(&mut self) {
        self.chain.clear();
        self.alternatives.clear();
        self.active_path = None;
        self.history.clear();
    }

    /// Get the number of steps in the chain
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Set metadata on the last step
    pub fn set_metadata(&mut self, key: String, value: String) {
        if let Some(step) = self.chain.last_mut() {
            step.metadata.insert(key, value);
        }
    }
}

impl fmt::Display for ChainOfThought {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, step) in self.chain.iter().enumerate() {
            writeln!(
                f,
                "[{}] {:?} (conf: {:.2})\n{}\n",
                i + 1,
                step.step_type,
                step.confidence,
                step.content
            )?;
        }
        Ok(())
    }
}

impl Default for ChainOfThought {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cot_creation() {
        let cot = ChainOfThought::new();
        assert!(cot.is_empty());
    }

    #[test]
    fn test_add_steps() {
        let mut cot = ChainOfThought::new();

        cot.observe("I see a problem".to_string(), 0.9);
        cot.reason("Let me think about this".to_string(), 0.8);
        cot.plan("Here's what I'll do".to_string(), 0.7);
        cot.execute("Doing it now".to_string(), 0.6);
        cot.verify("Looks good".to_string(), 0.8);
        cot.conclude("Problem solved".to_string(), 0.9);

        assert_eq!(cot.len(), 6);
    }

    #[test]
    fn test_overall_confidence() {
        let mut cot = ChainOfThought::new();

        cot.observe("Step 1".to_string(), 0.8);
        cot.reason("Step 2".to_string(), 0.7);
        cot.conclude("Done".to_string(), 0.9);

        let confidence = cot.overall_confidence();
        assert!(confidence > 0.0);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_rollback() {
        let mut cot = ChainOfThought::new();

        cot.observe("Step 1".to_string(), 0.8);
        cot.reason("Step 2".to_string(), 0.7);
        cot.conclude("Step 3".to_string(), 0.9);

        assert_eq!(cot.len(), 3);

        let result = cot.rollback(1);
        assert!(result);
        assert_eq!(cot.len(), 2);

        let result = cot.undo_rollback(1);
        assert!(result);
        assert_eq!(cot.len(), 3);
    }

    #[test]
    fn test_verify_chain() {
        let mut cot = ChainOfThought::new();

        cot.observe("Observation".to_string(), 0.9);
        cot.reason("Reasoning".to_string(), 0.8);

        let result = cot.verify_chain();
        assert!(!result.passed); // No conclusion

        cot.conclude("Conclusion".to_string(), 0.9);

        let result = cot.verify_chain();
        assert!(result.passed);
    }

    #[test]
    fn test_display() {
        let mut cot = ChainOfThought::new();
        cot.observe("Test".to_string(), 0.9);

        let s = cot.to_string();
        assert!(!s.is_empty());
    }
}
