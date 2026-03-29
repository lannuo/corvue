//! OpenAI HTTP client

use super::types::*;
use crate::error::ProviderError;
use futures::Stream;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::time::Duration;

/// Default API base URL
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// OpenAI API client
#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
    organization: Option<String>,
    project: Option<String>,
}

impl OpenAIClient {
    /// Create a new OpenAI client with an API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            organization: None,
            project: None,
        }
    }

    /// Create a new OpenAI client from environment variables
    ///
    /// Looks for `OPENAI_API_KEY`, `OPENAI_ORG_ID`, and `OPENAI_PROJECT_ID`.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Auth("OPENAI_API_KEY not set".to_string()))?;

        let mut client = Self::new(api_key);

        if let Ok(org) = std::env::var("OPENAI_ORG_ID") {
            client = client.with_organization(org);
        }

        if let Ok(project) = std::env::var("OPENAI_PROJECT_ID") {
            client = client.with_project(project);
        }

        Ok(client)
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set an organization ID
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Set a project ID
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Build a request with authentication headers
    fn request(&self, method: reqwest::Method, path: &str) -> RequestBuilder {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.request(method, &url)
            .bearer_auth(&self.api_key);

        if let Some(org) = &self.organization {
            req = req.header("OpenAI-Organization", org);
        }

        if let Some(project) = &self.project {
            req = req.header("OpenAI-Project", project);
        }

        req
    }

    /// Execute a request and parse the JSON response
    async fn execute<T: DeserializeOwned>(&self, req: RequestBuilder) -> Result<T, ProviderError> {
        let response = req.send().await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return if status.as_u16() == 429 {
                Err(ProviderError::RateLimitExceeded)
            } else if status.as_u16() == 401 {
                Err(ProviderError::Auth("Invalid API key".to_string()))
            } else {
                Err(ProviderError::ApiRequest(format!("{}: {}", status, text)))
            };
        }

        let result = response.json().await?;
        Ok(result)
    }

    /// Create a chat completion
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        let req = self.request(reqwest::Method::POST, "chat/completions")
            .json(&request);

        self.execute(req).await
    }

    /// Create embeddings
    pub async fn embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, ProviderError> {
        let req = self.request(reqwest::Method::POST, "embeddings")
            .json(&request);

        self.execute(req).await
    }

    /// Create a streaming chat completion
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionStreamResponse, ProviderError>> + Send>>, ProviderError> {
        let req = self.request(reqwest::Method::POST, "chat/completions")
            .json(&request);

        let response = req.send().await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return if status.as_u16() == 429 {
                Err(ProviderError::RateLimitExceeded)
            } else if status.as_u16() == 401 {
                Err(ProviderError::Auth("Invalid API key".to_string()))
            } else {
                Err(ProviderError::ApiRequest(format!("{}: {}", status, text)))
            };
        }

        // Parse SSE stream
        let stream = response.bytes_stream();

        let parsed_stream = futures::stream::unfold(
            (stream, Vec::new()),
            |(mut stream, mut buffer)| async move {
                use futures::StreamExt;

                loop {
                    // Try to parse from buffer first
                    if let Some((response, remaining)) = parse_sse_line(&buffer) {
                        buffer = remaining;
                        return Some((Ok(response), (stream, buffer)));
                    }

                    // Read more data
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            buffer.extend_from_slice(&bytes);
                        }
                        Some(Err(e)) => {
                            return Some((Err(ProviderError::ApiRequest(e.to_string())), (stream, buffer)));
                        }
                        None => {
                            // End of stream
                            return None;
                        }
                    }
                }
            },
        );

        Ok(Box::pin(parsed_stream))
    }
}

/// Parse an SSE line from the buffer
fn parse_sse_line(buffer: &[u8]) -> Option<(ChatCompletionStreamResponse, Vec<u8>)> {
    // Find a complete SSE event
    let mut start = 0;
    while start < buffer.len() {
        // Find double newline which indicates end of event
        if let Some(pos) = buffer[start..].windows(2).position(|w| w == b"\n\n") {
            let event_end = start + pos + 2;
            let event_data = &buffer[start..event_end];

            // Parse the event data
            if let Some(response) = parse_sse_event(event_data) {
                let remaining = buffer[event_end..].to_vec();
                return Some((response, remaining));
            }

            start = event_end;
        } else {
            break;
        }
    }

    None
}

/// Parse a single SSE event
fn parse_sse_event(data: &[u8]) -> Option<ChatCompletionStreamResponse> {
    let data_str = String::from_utf8_lossy(data);

    // Find "data: " lines
    for line in data_str.lines() {
        if let Some(stripped) = line.strip_prefix("data: ") {
            if stripped == "[DONE]" {
                return None;
            }
            if let Ok(response) = serde_json::from_str::<ChatCompletionStreamResponse>(stripped) {
                return Some(response);
            }
        }
    }

    None
}
