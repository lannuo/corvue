//! Corvus Providers - Multi-provider LLM support
//!
//! This crate provides implementations of corvus-core traits for various LLM providers.

#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow missing docs for release

pub mod openai;
pub mod ollama;
pub mod error;
#[cfg(test)]
mod tests;

pub use openai::{OpenAICompletionModel, OpenAIEmbeddingModel, OpenAIClient};
pub use ollama::{
    create_client, create_completion_model, create_embedding_model,
};
pub use error::ProviderError;
