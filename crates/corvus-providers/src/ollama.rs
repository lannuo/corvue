//! Ollama provider implementation
//!
//! Ollama is compatible with OpenAI's API, so we can reuse the OpenAI provider.

use crate::openai::{OpenAIClient, OpenAICompletionModel, OpenAIEmbeddingModel};
use std::sync::Arc;

/// Common Ollama model names
pub mod models {
    /// Llama 3 models
    pub const LLAMA3_8B: &str = "llama3:8b";
    pub const LLAMA3_70B: &str = "llama3:70b";

    /// Llama 3.1 models
    pub const LLAMA3_1_8B: &str = "llama3.1:8b";
    pub const LLAMA3_1_70B: &str = "llama3.1:70b";

    /// Mistral models
    pub const MISTRAL: &str = "mistral";
    pub const MISTRAL_7B: &str = "mistral:7b";

    /// Gemma models
    pub const GEMMA_2B: &str = "gemma:2b";
    pub const GEMMA_7B: &str = "gemma:7b";

    /// Code models
    pub const CODELLAMA_7B: &str = "codellama:7b";
    pub const CODELLAMA_13B: &str = "codellama:13b";

    /// Embedding models
    pub const NOMIC_EMBED_TEXT: &str = "nomic-embed-text";
    pub const MXBAI_EMBED_LARGE: &str = "mxbai-embed-large";
}

/// Create an Ollama client
pub fn create_client(base_url: impl Into<String>) -> OpenAIClient {
    OpenAIClient::new("ollama").with_base_url(base_url)
}

/// Create an Ollama completion model
pub fn create_completion_model(
    client: Arc<OpenAIClient>,
    model: impl Into<String>,
) -> OpenAICompletionModel {
    OpenAICompletionModel::new(client, model)
}

/// Create an Ollama embedding model
pub fn create_embedding_model(
    client: Arc<OpenAIClient>,
    model: impl Into<String>,
) -> OpenAIEmbeddingModel {
    OpenAIEmbeddingModel::new(client, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_names() {
        assert_eq!(models::LLAMA3_8B, "llama3:8b");
        assert_eq!(models::MISTRAL_7B, "mistral:7b");
    }
}
