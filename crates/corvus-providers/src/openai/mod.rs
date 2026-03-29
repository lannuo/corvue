//! OpenAI provider implementation

mod client;
mod completion;
mod embedding;
mod types;

pub use client::OpenAIClient;
pub use completion::OpenAICompletionModel;
pub use embedding::OpenAIEmbeddingModel;

/// Common OpenAI model names
pub mod models {
    // GPT-4 models
    pub const GPT_4O: &str = "gpt-4o";
    pub const GPT_4O_MINI: &str = "gpt-4o-mini";
    pub const GPT_4_TURBO: &str = "gpt-4-turbo";
    pub const GPT_4: &str = "gpt-4";

    // GPT-3.5 models
    pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";

    // Embedding models
    pub const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
    pub const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";
    pub const TEXT_EMBEDDING_ADA_002: &str = "text-embedding-ada-002";
}
