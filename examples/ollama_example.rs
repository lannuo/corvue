//! Example showing how to use Ollama with Corvus
//!
//! First, make sure Ollama is running locally:
//!   curl -fsSL https://ollama.com/install.sh | sh
//!   ollama serve
//!
//! Then pull a model:
//!   ollama pull llama3:8b
//!
//! Then run this example:
//!   cargo run --example ollama_example

use corvus_providers::ollama;
use corvus_providers::openai::OpenAICompletionModel;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Corvus + Ollama Example");
    println!("===================\n");

    // Create an Ollama client
    let client = Arc::new(ollama::create_client("http://localhost:11434/v1"));

    // Create a completion model using Llama 3 8B
    let model = ollama::create_completion_model(client, ollama::models::LLAMA3_8B);

    println!("Using model: {}", ollama::models::LLAMA3_8B);
    println!("Ollama URL: http://localhost:11434/v1");
    println!();

    println!("Available Ollama models:");
    println!("  - Llama 3: {}", ollama::models::LLAMA3_8B);
    println!("  - Llama 3 (70B): {}", ollama::models::LLAMA3_70B);
    println!("  - Llama 3.1: {}", ollama::models::LLAMA3_1_8B);
    println!("  - Mistral: {}", ollama::models::MISTRAL_7B);
    println!("  - Gemma: {}", ollama::models::GEMMA_7B);
    println!("  - CodeLlama: {}", ollama::models::CODELLAMA_7B);
    println!();

    println!("Available embedding models:");
    println!("  - Nomic Embed Text: {}", ollama::models::NOMIC_EMBED_TEXT);
    println!("  - MXBAI Embed Large: {}", ollama::models::MXBAI_EMBED_LARGE);
    println!();

    println!("Note: This example shows how to use Ollama with Corvus.");
    println!("To use in your own local models with Corvus CLI, you would need to:");
    println!("  1. Configure the provider in your config");
    println!("  2. Or use the programmatic API directly");
    println!();

    Ok(())
}
