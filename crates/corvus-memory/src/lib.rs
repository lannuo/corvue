//! Corvus Memory - Cognitive memory system
//!
//! This crate provides memory system implementations for Corvus,
//! including a simple in-memory store and the full TagMemo V7
//! cognitive architecture.

#![warn(missing_docs)]

pub mod simple;
pub mod tagmemo;
pub mod vector;

pub use simple::InMemoryMemory;
pub use tagmemo::{
    EpaAnalysis, EpaModule, LifParams, MemoryRecord, PyramidLevel, ResidualPyramid,
    SpikePropagation, TagEdge, TagMemoMemory, TagMemoStorage, TagMemoWave, TagNode, WaveParams,
    WaveQueryResult,
};
pub use vector::{KnnIndex, VectorEntry};
