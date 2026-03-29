//! Provider integration tests

#[cfg(test)]
mod provider_tests {
    use corvus_core::completion::*;
    use corvus_core::embedding::EmbeddingModel;
    use corvus_core::types::{Message, Role};

    #[test]
    fn test_openai_client_creation() {
        let _client = crate::openai::OpenAIClient::new("test_key");
        // Just verify it can be created
    }

    #[test]
    fn test_openai_client_with_base_url() {
        let _client = crate::openai::OpenAIClient::new("test_key")
            .with_base_url("http://localhost:8080/v1");
        // Just verify it can be created with custom base URL
    }

    #[test]
    fn test_openai_completion_model_creation() {
        let client = std::sync::Arc::new(crate::openai::OpenAIClient::new("test_key"));
        let model = crate::openai::OpenAICompletionModel::new(client.clone(), "gpt-4o");
        assert_eq!(model.model_name(), "gpt-4o");
    }

    #[test]
    fn test_openai_embedding_model_creation() {
        let client = std::sync::Arc::new(crate::openai::OpenAIClient::new("test_key"));
        let model = crate::openai::OpenAIEmbeddingModel::new(client.clone(), "text-embedding-3-small");
        assert_eq!(model.model_name(), "text-embedding-3-small");
        assert_eq!(model.ndims(), 1536);
    }

    #[test]
    fn test_openai_embedding_model_with_dimensions() {
        let client = std::sync::Arc::new(crate::openai::OpenAIClient::new("test_key"));
        let model = crate::openai::OpenAIEmbeddingModel::new(client.clone(), "text-embedding-3-small")
            .with_dimensions(512);
        assert_eq!(model.ndims(), 512);
    }

    #[test]
    fn test_ollama_client_creation() {
        let _client = crate::ollama::create_client("http://localhost:11434/v1");
        // Just verify it can be created
    }

    #[test]
    fn test_ollama_completion_model_creation() {
        let client = std::sync::Arc::new(crate::ollama::create_client("http://localhost:11434/v1"));
        let model = crate::ollama::create_completion_model(client.clone(), "llama3.1:8b");
        assert_eq!(model.model_name(), "llama3.1:8b");
    }

    #[test]
    fn test_completion_request_creation() {
        let messages = vec![Message::user("Hello!")];
        let request = CompletionRequest::new("gpt-4o", messages);
        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].content, "Hello!");
        assert_eq!(request.messages[0].role, Role::User);
    }

    #[test]
    fn test_completion_request_with_options() {
        let messages = vec![Message::user("Hello!")];
        let request = CompletionRequest::new("gpt-4o", messages)
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_top_p(0.9);

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.top_p, Some(0.9));
    }

    #[test]
    fn test_message_roles() {
        let system_msg = Message::system("You are a helpful assistant.");
        assert_eq!(system_msg.role, Role::System);

        let user_msg = Message::user("Hello!");
        assert_eq!(user_msg.role, Role::User);

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, Role::Assistant);
    }

    #[test]
    fn test_openai_model_names() {
        use crate::openai::models;
        assert_eq!(models::GPT_4O, "gpt-4o");
        assert_eq!(models::GPT_4O_MINI, "gpt-4o-mini");
        assert_eq!(models::GPT_3_5_TURBO, "gpt-3.5-turbo");
        assert_eq!(models::TEXT_EMBEDDING_3_SMALL, "text-embedding-3-small");
    }

    #[test]
    fn test_ollama_model_names() {
        use crate::ollama::models;
        assert_eq!(models::LLAMA3_8B, "llama3:8b");
        assert_eq!(models::LLAMA3_1_8B, "llama3.1:8b");
        assert_eq!(models::MISTRAL_7B, "mistral:7b");
        assert_eq!(models::CODELLAMA_7B, "codellama:7b");
    }

    #[test]
    fn test_provider_error_display() {
        let error = crate::error::ProviderError::ApiRequest("Test error".to_string());
        assert!(error.to_string().contains("Test error"));

        let auth_error = crate::error::ProviderError::Auth("Invalid key".to_string());
        assert!(auth_error.to_string().contains("Invalid key"));

        let rate_limit_error = crate::error::ProviderError::RateLimitExceeded;
        assert!(rate_limit_error.to_string().contains("Rate limit"));
    }
}
