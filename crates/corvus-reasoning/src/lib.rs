//! Corvus Reasoning - Reasoning and planning engine
//!
//! This crate provides reasoning capabilities for Corvus:
//! - Chain-of-Thought (CoT) - structured thinking steps
//! - Multi-step planning - task decomposition and execution
//! - AgentDream - simulation and hypothesis testing

#![warn(missing_docs)]

pub mod chain_of_thought;
pub mod planning;
pub mod agent_dream;

pub use chain_of_thought::{
    ChainOfThought, ThoughtStep, ThoughtStepType, VerificationResult, AlternativePath,
};
pub use planning::{
    Planner, Task, TaskStatus, Dependency, ExecutionPlan, ExecutionMonitor,
};
pub use agent_dream::{
    AgentDream, Simulation, Hypothesis, ExperienceReplay, Policy,
};

/// Reasoning error type
#[derive(Debug, thiserror::Error)]
pub enum ReasoningError {
    /// Invalid reasoning state
    #[error("Invalid reasoning state: {0}")]
    InvalidState(String),

    /// Thought generation failed
    #[error("Thought generation failed: {0}")]
    ThoughtGenerationFailed(String),

    /// Planning failed
    #[error("Planning failed: {0}")]
    PlanningFailed(String),

    /// Simulation error
    #[error("Simulation error: {0}")]
    SimulationError(String),

    /// General error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for reasoning operations
pub type Result<T> = std::result::Result<T, ReasoningError>;
