//! AgentDream - Simulation and hypothesis testing
//!
//! Provides simulated thinking, hypothesis推演, experience replay, and policy optimization.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// A single experience for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// Experience ID
    pub id: String,
    /// State before the action
    pub state_before: HashMap<String, f32>,
    /// Action taken
    pub action: String,
    /// State after the action
    pub state_after: HashMap<String, f32>,
    /// Reward received
    pub reward: f32,
    /// Whether this experience was successful
    pub success: bool,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Experience replay buffer
pub struct ExperienceReplay {
    /// Stored experiences
    experiences: VecDeque<Experience>,
    /// Maximum buffer size
    max_size: usize,
    /// Current sum of rewards
    reward_sum: f32,
    /// Current sum of squared rewards
    reward_sq_sum: f32,
}

/// A hypothesis to test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Hypothesis ID
    pub id: String,
    /// Hypothesis statement
    pub statement: String,
    /// Confidence in this hypothesis (0.0-1.0)
    pub confidence: f32,
    /// Predictions if this hypothesis is true
    pub predictions: Vec<String>,
    /// Evidence supporting this hypothesis
    pub evidence: Vec<String>,
    /// Evidence contradicting this hypothesis
    pub counterevidence: Vec<String>,
    /// Tests to validate this hypothesis
    pub tests: Vec<String>,
    /// Whether this hypothesis has been verified
    pub verified: bool,
    /// Timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    /// Simulation ID
    pub id: String,
    /// Simulation name/description
    pub name: String,
    /// Initial state
    pub initial_state: HashMap<String, f32>,
    /// Sequence of actions
    pub actions: Vec<String>,
    /// Final state
    pub final_state: HashMap<String, f32>,
    /// Total reward
    pub total_reward: f32,
    /// Whether the simulation was successful
    pub success: bool,
    /// Duration of the simulation
    pub duration: Option<Duration>,
    /// Timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Policy for action selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// State-action value estimates
    pub q_values: HashMap<String, HashMap<String, f32>>,
    /// Exploration rate (epsilon)
    pub epsilon: f32,
    /// Learning rate (alpha)
    pub alpha: f32,
    /// Discount factor (gamma)
    pub gamma: f32,
    /// Number of times this policy has been used
    pub usage_count: u64,
    /// Total reward accumulated
    pub total_reward: f32,
    /// Timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// AgentDream engine for simulation and learning
pub struct AgentDream {
    /// Experience replay buffer
    replay_buffer: ExperienceReplay,
    /// Active hypotheses
    hypotheses: HashMap<String, Hypothesis>,
    /// Simulation history
    simulations: Vec<Simulation>,
    /// Current policies
    policies: HashMap<String, Policy>,
    /// Active policy ID
    active_policy_id: Option<String>,
    /// Dream state
    dream_state: HashMap<String, f32>,
    /// Whether we're currently dreaming
    is_dreaming: bool,
}

impl ExperienceReplay {
    /// Create a new experience replay buffer
    pub fn new(max_size: usize) -> Self {
        Self {
            experiences: VecDeque::with_capacity(max_size),
            max_size,
            reward_sum: 0.0,
            reward_sq_sum: 0.0,
        }
    }

    /// Add an experience to the buffer
    pub fn add(&mut self, experience: Experience) {
        self.reward_sum += experience.reward;
        self.reward_sq_sum += experience.reward * experience.reward;

        if self.experiences.len() >= self.max_size {
            if let Some(removed) = self.experiences.pop_front() {
                self.reward_sum -= removed.reward;
                self.reward_sq_sum -= removed.reward * removed.reward;
            }
        }

        self.experiences.push_back(experience);
    }

    /// Sample a random experience
    pub fn sample(&self) -> Option<&Experience> {
        if self.experiences.is_empty() {
            return None;
        }

        let index = rand::random::<usize>() % self.experiences.len();
        self.experiences.get(index)
    }

    /// Sample multiple experiences
    pub fn sample_batch(&self, batch_size: usize) -> Vec<&Experience> {
        let mut result = Vec::with_capacity(batch_size.min(self.experiences.len()));

        for _ in 0..batch_size {
            if let Some(exp) = self.sample() {
                result.push(exp);
            }
        }

        result
    }

    /// Get all experiences
    pub fn all(&self) -> impl Iterator<Item = &Experience> {
        self.experiences.iter()
    }

    /// Get average reward
    pub fn average_reward(&self) -> f32 {
        if self.experiences.is_empty() {
            0.0
        } else {
            self.reward_sum / self.experiences.len() as f32
        }
    }

    /// Get reward variance
    pub fn reward_variance(&self) -> f32 {
        if self.experiences.is_empty() {
            0.0
        } else {
            let n = self.experiences.len() as f32;
            let mean = self.reward_sum / n;
            (self.reward_sq_sum / n - mean * mean).max(0.0)
        }
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.experiences.clear();
        self.reward_sum = 0.0;
        self.reward_sq_sum = 0.0;
    }
}

impl Policy {
    /// Create a new policy
    pub fn new(name: String, epsilon: f32, alpha: f32, gamma: f32) -> Self {
        Self {
            id: Self::generate_id(),
            name,
            q_values: HashMap::new(),
            epsilon: epsilon.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
            gamma: gamma.clamp(0.0, 1.0),
            usage_count: 0,
            total_reward: 0.0,
            created_at: chrono::Utc::now(),
        }
    }

    /// Generate a policy ID
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("policy_{:x}", nanos)
    }

    /// Get Q-value for a state-action pair
    pub fn get_q(&self, state: &str, action: &str) -> f32 {
        self.q_values
            .get(state)
            .and_then(|actions| actions.get(action))
            .copied()
            .unwrap_or(0.0)
    }

    /// Set Q-value for a state-action pair
    pub fn set_q(&mut self, state: &str, action: &str, value: f32) {
        self.q_values
            .entry(state.to_string())
            .or_default()
            .insert(action.to_string(), value);
    }

    /// Select an action using epsilon-greedy policy
    pub fn select_action(&self, state: &str, available_actions: &[String]) -> String {
        if available_actions.is_empty() {
            return String::new();
        }

        // Epsilon-greedy: explore with probability epsilon
        if rand::random::<f32>() < self.epsilon {
            let index = rand::random::<usize>() % available_actions.len();
            return available_actions[index].clone();
        }

        // Exploit: choose best action
        available_actions
            .iter()
            .max_by(|a, b| {
                let q_a = self.get_q(state, a);
                let q_b = self.get_q(state, b);
                q_a.partial_cmp(&q_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or_else(|| available_actions[0].clone())
    }

    /// Update Q-value using TD learning
    pub fn update(
        &mut self,
        state: &str,
        action: &str,
        reward: f32,
        next_state: &str,
        next_actions: &[String],
    ) {
        let old_q = self.get_q(state, action);

        // Find max Q-value for next state
        let max_next_q = next_actions
            .iter()
            .map(|a| self.get_q(next_state, a))
            .fold(f32::NEG_INFINITY, f32::max);

        let new_q = old_q + self.alpha * (reward + self.gamma * max_next_q - old_q);
        self.set_q(state, action, new_q);

        self.usage_count += 1;
        self.total_reward += reward;
    }

    /// Get the best action for a state
    pub fn best_action<'a>(&self, state: &str, available_actions: &'a [String]) -> Option<&'a str> {
        if available_actions.is_empty() {
            return None;
        }

        available_actions
            .iter()
            .max_by(|a, b| {
                let q_a = self.get_q(state, a);
                let q_b = self.get_q(state, b);
                q_a.partial_cmp(&q_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.as_str())
    }

    /// Decay epsilon (reduce exploration over time)
    pub fn decay_epsilon(&mut self, decay_factor: f32, min_epsilon: f32) {
        self.epsilon = (self.epsilon * decay_factor).max(min_epsilon);
    }
}

impl AgentDream {
    /// Create a new AgentDream engine
    pub fn new() -> Self {
        Self {
            replay_buffer: ExperienceReplay::new(1000),
            hypotheses: HashMap::new(),
            simulations: Vec::new(),
            policies: HashMap::new(),
            active_policy_id: None,
            dream_state: HashMap::new(),
            is_dreaming: false,
        }
    }

    /// Create with custom configuration
    pub fn with_config(replay_buffer_size: usize, _max_simulation_depth: usize) -> Self {
        Self {
            replay_buffer: ExperienceReplay::new(replay_buffer_size),
            hypotheses: HashMap::new(),
            simulations: Vec::new(),
            policies: HashMap::new(),
            active_policy_id: None,
            dream_state: HashMap::new(),
            is_dreaming: false,
        }
    }

    /// Add an experience to replay buffer
    pub fn add_experience(
        &mut self,
        state_before: HashMap<String, f32>,
        action: String,
        state_after: HashMap<String, f32>,
        reward: f32,
        success: bool,
    ) -> String {
        let id = format!("exp_{}", Self::generate_id());

        let experience = Experience {
            id: id.clone(),
            state_before,
            action,
            state_after,
            reward,
            success,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        self.replay_buffer.add(experience);
        id
    }

    /// Create a hypothesis
    pub fn create_hypothesis(
        &mut self,
        statement: String,
        confidence: f32,
    ) -> &Hypothesis {
        let id = format!("hyp_{}", Self::generate_id());

        let hypothesis = Hypothesis {
            id: id.clone(),
            statement,
            confidence: confidence.clamp(0.0, 1.0),
            predictions: Vec::new(),
            evidence: Vec::new(),
            counterevidence: Vec::new(),
            tests: Vec::new(),
            verified: false,
            created_at: chrono::Utc::now(),
        };

        self.hypotheses.insert(id.clone(), hypothesis);
        self.hypotheses.get(&id).unwrap()
    }

    /// Add evidence to a hypothesis
    pub fn add_evidence(&mut self, hypothesis_id: &str, evidence: String) -> bool {
        if let Some(hyp) = self.hypotheses.get_mut(hypothesis_id) {
            hyp.evidence.push(evidence);
            hyp.confidence = (hyp.confidence + 0.1).min(1.0);
            return true;
        }
        false
    }

    /// Add counterevidence to a hypothesis
    pub fn add_counterevidence(&mut self, hypothesis_id: &str, evidence: String) -> bool {
        if let Some(hyp) = self.hypotheses.get_mut(hypothesis_id) {
            hyp.counterevidence.push(evidence);
            hyp.confidence = (hyp.confidence - 0.1).max(0.0);
            return true;
        }
        false
    }

    /// Verify a hypothesis
    pub fn verify_hypothesis(&mut self, hypothesis_id: &str, verified: bool) -> bool {
        if let Some(hyp) = self.hypotheses.get_mut(hypothesis_id) {
            hyp.verified = true;
            hyp.confidence = if verified { 0.9 } else { 0.1 };
            return true;
        }
        false
    }

    /// Get all hypotheses
    pub fn hypotheses(&self) -> impl Iterator<Item = &Hypothesis> {
        self.hypotheses.values()
    }

    /// Get the most confident hypothesis
    pub fn best_hypothesis(&self) -> Option<&Hypothesis> {
        self.hypotheses
            .values()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Create a policy
    pub fn create_policy(&mut self, name: String, epsilon: f32, alpha: f32, gamma: f32) -> &Policy {
        let policy = Policy::new(name, epsilon, alpha, gamma);
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);

        if self.active_policy_id.is_none() {
            self.active_policy_id = Some(id.clone());
        }

        self.policies.get(&id).unwrap()
    }

    /// Set active policy
    pub fn set_active_policy(&mut self, policy_id: &str) -> bool {
        if self.policies.contains_key(policy_id) {
            self.active_policy_id = Some(policy_id.to_string());
            return true;
        }
        false
    }

    /// Get active policy
    pub fn active_policy(&self) -> Option<&Policy> {
        self.active_policy_id
            .as_ref()
            .and_then(|id| self.policies.get(id))
    }

    /// Get active policy (mutable)
    pub fn active_policy_mut(&mut self) -> Option<&mut Policy> {
        self.active_policy_id
            .as_ref()
            .and_then(|id| self.policies.get_mut(id))
    }

    /// Run a simulation
    pub fn simulate(
        &mut self,
        name: String,
        initial_state: HashMap<String, f32>,
        actions: Vec<String>,
    ) -> &Simulation {
        let id = format!("sim_{}", Self::generate_id());
        let start = Instant::now();

        // Simple simulation: just track state changes
        let mut final_state = initial_state.clone();
        let mut total_reward = 0.0;

        for (i, _action) in actions.iter().enumerate() {
            // Simple state update logic
            final_state.insert(format!("action_{}", i), 1.0);
            total_reward += 0.1; // Small positive reward for each step
        }

        let simulation = Simulation {
            id: id.clone(),
            name,
            initial_state,
            actions,
            final_state,
            total_reward,
            success: true,
            duration: Some(start.elapsed()),
            created_at: chrono::Utc::now(),
        };

        self.simulations.push(simulation);
        self.simulations.last().unwrap()
    }

    /// Start dreaming
    pub fn start_dream(&mut self, initial_state: HashMap<String, f32>) {
        self.dream_state = initial_state;
        self.is_dreaming = true;
    }

    /// Stop dreaming
    pub fn stop_dream(&mut self) {
        self.is_dreaming = false;
    }

    /// Check if dreaming
    pub fn is_dreaming(&self) -> bool {
        self.is_dreaming
    }

    /// Get current dream state
    pub fn dream_state(&self) -> &HashMap<String, f32> {
        &self.dream_state
    }

    /// Learn from replay buffer
    pub fn learn_from_replay(&mut self, batch_size: usize) {
        // Clone the batch to avoid borrow conflicts
        let batch: Vec<_> = self.replay_buffer.sample_batch(batch_size)
            .into_iter()
            .cloned()
            .collect();

        if let Some(policy) = self.active_policy_mut() {
            for exp in batch {
                // Convert state to string representation (simplified)
                let state_before = Self::state_to_string(&exp.state_before);
                let state_after = Self::state_to_string(&exp.state_after);

                // Get available actions (simplified)
                let actions = vec![exp.action.clone()];

                policy.update(
                    &state_before,
                    &exp.action,
                    exp.reward,
                    &state_after,
                    &actions,
                );
            }
        }
    }

    /// Convert state to string representation
    fn state_to_string(state: &HashMap<String, f32>) -> String {
        let mut keys: Vec<_> = state.keys().collect();
        keys.sort();

        let mut result = String::new();
        for key in keys {
            result.push_str(&format!("{}:{:.2},", key, state.get(key).unwrap_or(&0.0)));
        }
        result
    }

    /// Get replay buffer
    pub fn replay_buffer(&self) -> &ExperienceReplay {
        &self.replay_buffer
    }

    /// Get simulation history
    pub fn simulations(&self) -> &[Simulation] {
        &self.simulations
    }

    /// Get all policies
    pub fn policies(&self) -> impl Iterator<Item = &Policy> {
        self.policies.values()
    }

    /// Generate an experience ID
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{:x}", nanos)
    }

    /// Clear everything
    pub fn clear(&mut self) {
        self.replay_buffer.clear();
        self.hypotheses.clear();
        self.simulations.clear();
        self.policies.clear();
        self.active_policy_id = None;
        self.dream_state.clear();
        self.is_dreaming = false;
    }
}

impl Default for AgentDream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_replay() {
        let mut buffer = ExperienceReplay::new(10);
        assert!(buffer.is_empty());

        let exp = Experience {
            id: "test".to_string(),
            state_before: HashMap::new(),
            action: "test".to_string(),
            state_after: HashMap::new(),
            reward: 1.0,
            success: true,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        buffer.add(exp);
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new("test".to_string(), 0.1, 0.5, 0.9);
        assert_eq!(policy.name, "test");
        assert!(policy.q_values.is_empty());
    }

    #[test]
    fn test_policy_q_values() {
        let mut policy = Policy::new("test".to_string(), 0.1, 0.5, 0.9);

        policy.set_q("state1", "action1", 0.5);
        assert_eq!(policy.get_q("state1", "action1"), 0.5);
    }

    #[test]
    fn test_policy_select_action() {
        let mut policy = Policy::new("test".to_string(), 0.0, 0.5, 0.9); // No exploration

        policy.set_q("state1", "action1", 1.0);
        policy.set_q("state1", "action2", 0.5);

        let actions = vec!["action1".to_string(), "action2".to_string()];
        let selected = policy.select_action("state1", &actions);

        // Should always select action1 (higher Q-value)
        assert_eq!(selected, "action1");
    }

    #[test]
    fn test_agent_dream_creation() {
        let dream = AgentDream::new();
        assert!(!dream.is_dreaming());
    }

    #[test]
    fn test_add_experience() {
        let mut dream = AgentDream::new();
        let id = dream.add_experience(
            HashMap::new(),
            "test".to_string(),
            HashMap::new(),
            1.0,
            true,
        );

        assert!(!id.is_empty());
        assert_eq!(dream.replay_buffer().len(), 1);
    }

    #[test]
    fn test_create_hypothesis() {
        let mut dream = AgentDream::new();
        let hyp = dream.create_hypothesis("This is a test".to_string(), 0.8);

        assert_eq!(hyp.statement, "This is a test");
        assert_eq!(hyp.confidence, 0.8);
    }

    #[test]
    fn test_create_policy() {
        let mut dream = AgentDream::new();
        let policy = dream.create_policy("test".to_string(), 0.1, 0.5, 0.9);

        assert_eq!(policy.name, "test");
        assert!(dream.active_policy().is_some());
    }

    #[test]
    fn test_simulate() {
        let mut dream = AgentDream::new();

        let initial_state = HashMap::new();
        let actions = vec!["step1".to_string(), "step2".to_string()];

        let sim = dream.simulate("test sim".to_string(), initial_state, actions);

        assert_eq!(sim.name, "test sim");
        assert!(sim.success);
    }

    #[test]
    fn test_dreaming() {
        let mut dream = AgentDream::new();

        let mut state = HashMap::new();
        state.insert("key".to_string(), 1.0);

        dream.start_dream(state);
        assert!(dream.is_dreaming());

        dream.stop_dream();
        assert!(!dream.is_dreaming());
    }
}
