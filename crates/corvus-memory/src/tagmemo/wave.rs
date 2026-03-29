//! TagMemo Wave algorithm
//!
//! Implements N-hop spike propagation with co-occurrence matrix,
//! core/normal tag distinction, and LIF (Leaky Integrate-and-Fire) neurons.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

/// A tag node in the memory network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagNode {
    /// Unique tag identifier
    pub id: String,
    /// Tag name/content
    pub tag: String,
    /// Whether this is a core tag
    pub is_core: bool,
    /// Node activation level (for LIF)
    pub activation: f32,
    /// Membrane potential (for LIF)
    pub membrane_potential: f32,
    /// Last spike time (milliseconds since epoch)
    pub last_spike_ms: Option<u64>,
    /// Embedding vector (optional)
    pub embedding: Option<Vec<f32>>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// An edge between tag nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Edge weight (co-occurrence strength)
    pub weight: f32,
    /// Number of co-occurrences
    pub cooccurrence_count: u64,
    /// Edge type
    pub edge_type: EdgeType,
}

/// Type of edge between tags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Semantic similarity
    Semantic,
    /// Temporal co-occurrence
    Temporal,
    /// Hierarchical (parent-child)
    Hierarchical,
    /// Associative (general)
    Associative,
}

/// Spike propagation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikePropagation {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Spike time (milliseconds since epoch)
    pub time_ms: u64,
    /// Spike magnitude
    pub magnitude: f32,
    /// Hop count
    pub hop: usize,
}

/// Result of a wave propagation query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveQueryResult {
    /// Activated nodes sorted by activation
    pub activated_nodes: Vec<(String, f32)>,
    /// Spike propagation events
    pub spikes: Vec<SpikePropagation>,
    /// Total hops traversed
    pub total_hops: usize,
    /// Query duration in milliseconds
    pub duration_ms: u64,
}

/// TagMemo Wave network
#[derive(Debug, Clone)]
pub struct TagMemoWave {
    /// Tag nodes by ID
    pub(crate) nodes: HashMap<String, TagNode>,
    /// Tag name to ID mapping
    pub(crate) tag_to_id: HashMap<String, String>,
    /// Outgoing edges
    pub(crate) outgoing_edges: HashMap<String, Vec<TagEdge>>,
    /// Incoming edges
    pub(crate) incoming_edges: HashMap<String, Vec<TagEdge>>,
    /// Co-occurrence matrix (sparse)
    pub(crate) cooccurrence: HashMap<(String, String), u64>,
    /// LIF neuron parameters
    lif_params: LifParams,
    /// Wave propagation parameters
    wave_params: WaveParams,
    /// Next node ID
    pub(crate) next_node_id: u64,
}

/// LIF (Leaky Integrate-and-Fire) neuron parameters
#[derive(Debug, Clone, Copy)]
pub struct LifParams {
    /// Resting potential
    pub resting_potential: f32,
    /// Threshold potential
    pub threshold: f32,
    /// Reset potential
    pub reset_potential: f32,
    /// Membrane time constant (ms)
    pub tau: f32,
    /// Refractory period (ms)
    pub refractory_period: f32,
    /// Spike amplitude
    pub spike_amplitude: f32,
}

/// Wave propagation parameters
#[derive(Debug, Clone, Copy)]
pub struct WaveParams {
    /// Maximum number of hops
    pub max_hops: usize,
    /// Decay factor per hop
    pub decay_per_hop: f32,
    /// Activation threshold
    pub activation_threshold: f32,
    /// Maximum activated nodes
    pub max_activated: usize,
    /// Spike propagation speed
    pub propagation_speed: f32,
}

impl Default for LifParams {
    fn default() -> Self {
        Self {
            resting_potential: -70.0,
            threshold: -50.0,
            reset_potential: -65.0,
            tau: 10.0,
            refractory_period: 2.0,
            spike_amplitude: 1.0,
        }
    }
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            max_hops: 5,
            decay_per_hop: 0.7,
            activation_threshold: 0.01,
            max_activated: 100,
            propagation_speed: 1.0,
        }
    }
}

impl TagMemoWave {
    /// Create a new TagMemo Wave network
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            tag_to_id: HashMap::new(),
            outgoing_edges: HashMap::new(),
            incoming_edges: HashMap::new(),
            cooccurrence: HashMap::new(),
            lif_params: LifParams::default(),
            wave_params: WaveParams::default(),
            next_node_id: 1,
        }
    }

    /// Get current time in milliseconds
    fn now_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }

    /// Add a tag node
    pub fn add_tag(&mut self, tag: String, is_core: bool) -> String {
        if let Some(id) = self.tag_to_id.get(&tag) {
            return id.clone();
        }

        let id = format!("tag_{}", self.next_node_id);
        self.next_node_id += 1;

        let node = TagNode {
            id: id.clone(),
            tag: tag.clone(),
            is_core,
            activation: 0.0,
            membrane_potential: self.lif_params.resting_potential,
            last_spike_ms: None,
            embedding: None,
            metadata: HashMap::new(),
        };

        self.nodes.insert(id.clone(), node);
        self.tag_to_id.insert(tag, id.clone());

        id
    }

    /// Get a tag node by ID
    pub fn get_node(&self, id: &str) -> Option<&TagNode> {
        self.nodes.get(id)
    }

    /// Get a tag node by tag name
    pub fn get_node_by_tag(&self, tag: &str) -> Option<&TagNode> {
        self.tag_to_id.get(tag).and_then(|id| self.nodes.get(id))
    }

    /// Add an edge between tags
    pub fn add_edge(&mut self, source: &str, target: &str, edge_type: EdgeType, weight: f32) {
        let source_id = if let Some(id) = self.tag_to_id.get(source) {
            id.clone()
        } else {
            self.add_tag(source.to_string(), false)
        };

        let target_id = if let Some(id) = self.tag_to_id.get(target) {
            id.clone()
        } else {
            self.add_tag(target.to_string(), false)
        };

        let edge = TagEdge {
            source: source_id.clone(),
            target: target_id.clone(),
            weight,
            cooccurrence_count: 1,
            edge_type,
        };

        self.outgoing_edges.entry(source_id.clone()).or_default().push(edge.clone());
        self.incoming_edges.entry(target_id.clone()).or_default().push(edge);

        // Update co-occurrence
        let key = if source_id < target_id {
            (source_id, target_id)
        } else {
            (target_id, source_id)
        };
        *self.cooccurrence.entry(key).or_insert(0) += 1;
    }

    /// Record co-occurrence of multiple tags
    pub fn record_cooccurrence(&mut self, tags: &[String], edge_type: EdgeType) {
        for i in 0..tags.len() {
            for j in (i + 1)..tags.len() {
                let weight = 1.0 / (tags.len() as f32 - 1.0);
                self.add_edge(&tags[i], &tags[j], edge_type, weight);
            }
        }
    }

    /// Set embedding for a tag
    pub fn set_embedding(&mut self, tag: &str, embedding: Vec<f32>) {
        let tag_id = self.tag_to_id.get(tag).cloned();
        if let Some(id) = tag_id {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.embedding = Some(embedding);
            }
        }
    }

    /// Propagate a wave from starting tags
    pub fn propagate_wave(&mut self, start_tags: &[String], initial_activation: f32) -> WaveQueryResult {
        let start_time = Self::now_ms();
        let mut spikes = Vec::new();
        let mut activated_nodes = HashMap::new();
        let mut queue = VecDeque::new();

        // Initialize with start tags
        for tag in start_tags {
            if let Some(id) = self.tag_to_id.get(tag) {
                activated_nodes.insert(id.clone(), initial_activation);
                queue.push_back((id.clone(), initial_activation, 0));

                // Stimulate the node
                let now = Self::now_ms();
                let lif_params = self.lif_params;
                if let Some(node) = self.nodes.get_mut(id) {
                    node.activation = initial_activation;
                    if Self::lif_stimulate_static(node, initial_activation, now, lif_params) {
                        spikes.push(SpikePropagation {
                            source: id.clone(),
                            target: id.clone(),
                            time_ms: now,
                            magnitude: initial_activation,
                            hop: 0,
                        });
                    }
                }
            }
        }

        let mut total_hops = 0;

        // BFS propagation
        while let Some((node_id, magnitude, hop)) = queue.pop_front() {
            if hop >= self.wave_params.max_hops {
                continue;
            }

            total_hops = total_hops.max(hop);

            let outgoing = self.outgoing_edges.get(&node_id).cloned().unwrap_or_default();

            for edge in outgoing {
                let new_magnitude = magnitude * edge.weight * self.wave_params.decay_per_hop;

                if new_magnitude < self.wave_params.activation_threshold {
                    continue;
                }

                let current_activation = activated_nodes.entry(edge.target.clone()).or_insert(0.0);
                *current_activation += new_magnitude;

                // Update node activation
                let now = Self::now_ms();
                let lif_params = self.lif_params;
                if let Some(target_node) = self.nodes.get_mut(&edge.target) {
                    target_node.activation = *current_activation;

                    // Check for spike
                    if Self::lif_stimulate_static(target_node, new_magnitude, now, lif_params) {
                        spikes.push(SpikePropagation {
                            source: node_id.clone(),
                            target: edge.target.clone(),
                            time_ms: now,
                            magnitude: new_magnitude,
                            hop: hop + 1,
                        });

                        queue.push_back((edge.target.clone(), new_magnitude, hop + 1));
                    }
                }
            }
        }

        // Sort activated nodes
        let mut activated_nodes: Vec<_> = activated_nodes.into_iter().collect();
        activated_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        activated_nodes.truncate(self.wave_params.max_activated);

        let end_time = Self::now_ms();

        WaveQueryResult {
            activated_nodes,
            spikes,
            total_hops,
            duration_ms: end_time - start_time,
        }
    }

    /// Get activated tags with their activation levels
    pub fn get_activated_tags(&self, result: &WaveQueryResult) -> Vec<(String, f32)> {
        result.activated_nodes.iter()
            .filter_map(|(id, activation)| {
                self.nodes.get(id).map(|node| (node.tag.clone(), *activation))
            })
            .collect()
    }

    /// Get similar tags using wave propagation
    pub fn find_similar_tags(&mut self, tag: &str, top_k: usize) -> Vec<(String, f32)> {
        let result = self.propagate_wave(&[tag.to_string()], 1.0);
        let mut tags = self.get_activated_tags(&result);

        // Remove the input tag itself
        tags.retain(|(t, _)| t != tag);
        tags.truncate(top_k);

        tags
    }

    /// Reset all node activations
    pub fn reset_activations(&mut self) {
        for node in self.nodes.values_mut() {
            node.activation = 0.0;
            node.membrane_potential = self.lif_params.resting_potential;
            node.last_spike_ms = None;
        }
    }

    /// Get core tags
    pub fn core_tags(&self) -> Vec<&TagNode> {
        self.nodes.values()
            .filter(|node| node.is_core)
            .collect()
    }

    /// Get all tags
    pub fn all_tags(&self) -> Vec<&TagNode> {
        self.nodes.values().collect()
    }

    // LIF neuron simulation

    fn lif_stimulate_static(node: &mut TagNode, input_current: f32, now_ms: u64, lif_params: LifParams) -> bool {
        // Check refractory period
        if let Some(last_spike) = node.last_spike_ms {
            if (now_ms - last_spike) < lif_params.refractory_period as u64 {
                return false;
            }
        }

        // Leak the membrane potential
        let time_since_last = if let Some(last_spike) = node.last_spike_ms {
            (now_ms - last_spike) as f32
        } else {
            lif_params.tau
        };

        let leak_factor = (-time_since_last / lif_params.tau).exp();
        node.membrane_potential = lif_params.resting_potential +
            (node.membrane_potential - lif_params.resting_potential) * leak_factor;

        // Add input current
        node.membrane_potential += input_current * 20.0; // Scale for LIF units

        // Check for spike
        if node.membrane_potential >= lif_params.threshold {
            node.membrane_potential = lif_params.reset_potential;
            node.last_spike_ms = Some(now_ms);
            true
        } else {
            false
        }
    }

    /// Set LIF parameters
    pub fn set_lif_params(&mut self, params: LifParams) {
        self.lif_params = params;
    }

    /// Set wave parameters
    pub fn set_wave_params(&mut self, params: WaveParams) {
        self.wave_params = params;
    }
}

impl Default for TagMemoWave {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_tag() {
        let mut wave = TagMemoWave::new();
        let id = wave.add_tag("test".to_string(), true);

        let node = wave.get_node(&id).unwrap();
        assert_eq!(node.tag, "test");
        assert!(node.is_core);
    }

    #[test]
    fn test_wave_propagation() {
        let mut wave = TagMemoWave::new();
        wave.add_tag("A".to_string(), true);
        wave.add_tag("B".to_string(), false);
        wave.add_tag("C".to_string(), false);

        wave.add_edge("A", "B", EdgeType::Associative, 0.8);
        wave.add_edge("B", "C", EdgeType::Associative, 0.6);

        let result = wave.propagate_wave(&["A".to_string()], 1.0);
        assert!(!result.activated_nodes.is_empty());
    }
}
