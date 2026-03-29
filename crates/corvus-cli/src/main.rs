//! Corvus CLI main entry point

use clap::Parser;
use console::style;
use corvus_cli::cache::CachedCompletionModel;
use corvus_cli::chat_memory::ChatMemory;
use corvus_cli::cli::{Cli, Commands, ConfigCommands, McpCommands, ModelCommands, SessionCommands, PluginCommands, MemoryCommands};
use corvus_cli::config::{Config, ConfigWizard, ProviderType};
use corvus_cli::errors::print_error;
use corvus_cli::format::{format_response, StreamingResponseHandler};
use corvus_cli::memory_store::TagMemoStore;
use corvus_cli::session::{SessionExport, SessionStorage};
use corvus_core::agent::Agent;
use corvus_core::completion::CompletionModel;
use corvus_memory::InMemoryMemory;
use corvus_providers::openai::{OpenAIClient, OpenAICompletionModel};
use corvus_providers::ollama;
use corvus_tools::{ExecuteTool, FileTool, GitTool, SearchTool, ShellTool, HttpTool, SystemTool};
use corvus_plugin::PluginManager;
use futures::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Run interactive chat loop with history and tab completion
async fn run_chat_interactive(
    agent: Agent,
    mut storage: SessionStorage,
    session_id: &str,
    mut chat_memory: Option<ChatMemory>,
) -> anyhow::Result<()> {
    let mut rl = Editor::<(), DefaultHistory>::new()?;

    // Load history from previous messages in this session
    let messages = storage.get_messages(session_id)?;
    for msg in messages {
        if msg.role == "user" {
            let _ = rl.add_history_entry(&msg.content);
        }
    }

    loop {
        let readline = rl.readline("You: ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    break;
                }

                let _ = rl.add_history_entry(input);
                storage.add_message(session_id, "user", input)?;

                // Save user message to memory and inject memories
                let enhanced_prompt = if let Some(memory) = &mut chat_memory {
                    memory.save_user_message(input).await?;
                    memory.inject_memories(input).await?
                } else {
                    input.to_string()
                };

                // Try streaming first, fall back to regular mode
                match run_with_streaming(&agent, &enhanced_prompt, &mut storage, session_id, &mut chat_memory).await {
                    Ok(_) => {}
                    Err(_) => {
                        // Fall back to non-streaming mode
                        match agent.run(&enhanced_prompt).await {
                            Ok(response) => {
                                println!("\n{}", style("Corvus:").blue().bold());
                                format_response(&response);
                                println!();
                                storage.add_message(session_id, "assistant", &response)?;

                                // Save assistant response to memory
                                if let Some(memory) = &mut chat_memory {
                                    memory.save_assistant_message(&response).await?;
                                }
                            }
                            Err(e) => print_error(&e.into()),
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

/// Run agent with streaming output
async fn run_with_streaming(
    agent: &Agent,
    input: &str,
    storage: &mut SessionStorage,
    session_id: &str,
    chat_memory: &mut Option<ChatMemory>,
) -> anyhow::Result<()> {
    let mut stream = agent.run_stream(input).await?;
    let mut handler = StreamingResponseHandler::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(delta) => {
                handler.handle_delta(&delta);
            }
            Err(e) => {
                handler.finish();
                return Err(e.into());
            }
        }
    }

    handler.finish();

    let full_response = handler.content().to_string();
    if !full_response.is_empty() {
        storage.add_message(session_id, "assistant", &full_response)?;

        // Save assistant response to memory
        if let Some(memory) = chat_memory {
            memory.save_assistant_message(&full_response).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        print_error(&e);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging
    if cli.verbose {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .init();
    }

    match &cli.command {
        Commands::Setup => {
            ConfigWizard::run().await?;
            return Ok(());
        }
        Commands::Session(session_cmd) => {
            return handle_session_commands(session_cmd).await;
        }
        Commands::Model(model_cmd) => {
            return handle_model_commands(model_cmd).await;
        }
        Commands::Config(config_cmd) => {
            return handle_config_commands(config_cmd).await;
        }
        Commands::Mcp(mcp_cmd) => {
            return handle_mcp_commands(mcp_cmd).await;
        }
        Commands::Plugin(plugin_cmd) => {
            return handle_plugin_commands(plugin_cmd).await;
        }
        Commands::Memory(memory_cmd) => {
            return handle_memory_commands(memory_cmd).await;
        }
        _ => {}
    }

    // Load config
    let mut config = Config::load();

    // Quick setup if no provider configured
    let needs_setup = match config.provider {
        ProviderType::OpenAI => {
            config.openai_api_key.is_none() && std::env::var("OPENAI_API_KEY").is_err()
        }
        ProviderType::Ollama => {
            config.ollama_base_url.is_none()
        }
    };

    if needs_setup {
        println!("No provider configured. Let's set it up quickly!\n");
        config = ConfigWizard::quick_setup().await?;
        println!();
    }

    // Create completion model based on provider
    let completion_model = create_completion_model(&config).await?;

    // Handle commands
    match cli.command {
        Commands::Chat(args) => {
            run_chat_mode(args, config, completion_model).await?;
        }
        Commands::Run(args) => {
            run_single_task(args, config, completion_model).await?;
        }
        Commands::Config(_) | Commands::Setup | Commands::Session(_) | Commands::Model(_) | Commands::Mcp(_) | Commands::Plugin(_) | Commands::Memory(_) => {
            unreachable!("These commands should have been handled earlier");
        }
    }

    Ok(())
}

async fn handle_session_commands(cmd: &SessionCommands) -> anyhow::Result<()> {
    let mut storage = SessionStorage::open_default()?;

    match cmd {
        SessionCommands::List(args) => {
            let sessions = storage.list_sessions(Some(args.limit))?;
            if sessions.is_empty() {
                println!("No sessions found.");
                return Ok(());
            }

            println!("{}", style("Recent Sessions:").bold());
            for session in sessions {
                let date = session.updated_at.format("%Y-%m-%d %H:%M");
                println!(
                    "  {} {} - {} ({} messages)",
                    style(&session.id).dim(),
                    style(date).cyan(),
                    style(&session.name).bold(),
                    session.message_count
                );
            }
        }
        SessionCommands::Continue(args) => {
            let session_id = if let Some(id) = &args.session_id {
                id.clone()
            } else if let Some(last) = storage.get_last_session()? {
                last.id
            } else {
                println!("No sessions found to continue.");
                return Ok(());
            };

            let messages = storage.get_messages(&session_id)?;
            if !messages.is_empty() {
                println!("{}", style("Previous conversation:").dim());
                for msg in &messages {
                    let role_str = match msg.role.as_str() {
                        "user" => "You:".to_string(),
                        "assistant" => "Corvus:".to_string(),
                        _ => format!("{}:", msg.role),
                    };
                    let role = match msg.role.as_str() {
                        "user" => style(role_str).green().bold(),
                        "assistant" => style(role_str).blue().bold(),
                        _ => style(role_str).dim(),
                    };
                    println!("{} {}", role, msg.content);
                }
                println!();
            }

            let config = Config::load();
            let completion_model = create_completion_model(&config).await?;
            let model_name = config.default_model.clone();

            let preamble = "You are Corvus, an intelligent AI assistant. You help with coding tasks and general questions.";
            let (agent, mcp_tool_count) = build_agent(&config, completion_model, preamble).await?;

            println!("Continuing session: {}\n", style(&session_id).dim());
            println!("Welcome back! (type 'exit' to quit)\n");
            println!("Using model: {}", model_name);
            if mcp_tool_count > 0 {
                println!("MCP tools loaded: {}", mcp_tool_count);
            }
            println!();

            run_chat_interactive(agent, storage, &session_id, None).await?;

            println!("Goodbye!");
        }
        SessionCommands::Show(args) => {
            if let Some(session) = storage.get_session(&args.session_id)? {
                println!("Session: {}", style(&session.name).bold());
                println!("ID: {}", style(&session.id).dim());
                println!(
                    "Created: {}",
                    session.created_at.format("%Y-%m-%d %H:%M:%S")
                );
                println!(
                    "Updated: {}",
                    session.updated_at.format("%Y-%m-%d %H:%M:%S")
                );
                println!("Messages: {}\n", session.message_count);

                let messages = storage.get_messages(&args.session_id)?;
                for msg in messages {
                    let role_str = match msg.role.as_str() {
                        "user" => "You:".to_string(),
                        "assistant" => "Corvus:".to_string(),
                        _ => format!("{}:", msg.role),
                    };
                    let role = match msg.role.as_str() {
                        "user" => style(role_str).green().bold(),
                        "assistant" => style(role_str).blue().bold(),
                        _ => style(role_str).dim(),
                    };
                    println!("{} {}", role, msg.content);
                    println!();
                }
            } else {
                println!("Session not found: {}", args.session_id);
            }
        }
        SessionCommands::Rename(args) => {
            if storage.rename_session(&args.session_id, &args.new_name)? {
                println!("Session renamed successfully.");
            } else {
                println!("Session not found: {}", args.session_id);
            }
        }
        SessionCommands::Delete(args) => {
            if storage.delete_session(&args.session_id)? {
                println!("Session deleted successfully.");
            } else {
                println!("Session not found: {}", args.session_id);
            }
        }
        SessionCommands::Search(args) => {
            let results = storage.search_sessions(&args.query, args.limit)?;
            if results.is_empty() {
                println!("No sessions found for: {}", args.query);
                return Ok(());
            }

            println!("{}", style("Search Results:").bold());
            for session in results {
                let date = session.updated_at.format("%Y-%m-%d %H:%M");
                println!(
                    "  {} {} - {} ({} messages)",
                    style(&session.id).dim(),
                    style(date).cyan(),
                    style(&session.name).bold(),
                    session.message_count
                );
            }
        }
        SessionCommands::Export(args) => {
            let export = storage.export_session(&args.session_id)?;

            let output_path = if let Some(output) = &args.output {
                output.clone()
            } else {
                std::path::PathBuf::from(format!("session-{}.json", export.session.id))
            };

            let content = serde_json::to_string_pretty(&export)?;
            std::fs::write(&output_path, content)?;

            println!("{} Session exported to: {}", style("✓").green().bold(), output_path.display());
        }
        SessionCommands::Import(args) => {
            let content = std::fs::read_to_string(&args.input)?;
            let export: SessionExport = serde_json::from_str(&content)?;

            let session = storage.import_session(export)?;
            println!("{} Session imported successfully!", style("✓").green().bold());
            println!("  New session ID: {}", style(&session.id).dim());
            println!("  Name: {}", style(&session.name).bold());
            println!("  Messages: {}", session.message_count);
        }
    }

    Ok(())
}

async fn handle_model_commands(cmd: &ModelCommands) -> anyhow::Result<()> {
    let config = Config::load();

    match cmd {
        ModelCommands::List => {
            println!("{}", style("Available Models:").bold());

            println!("\n{}", style("OpenAI Models:").dim());
            let openai_models = vec![
                ("gpt-4o", "Latest GPT-4 model (Recommended)"),
                ("gpt-4o-mini", "Fast and affordable"),
                ("gpt-4-turbo", "GPT-4 Turbo"),
                ("gpt-3.5-turbo", "GPT-3.5 Turbo"),
            ];
            for (model, desc) in openai_models {
                println!("  {} - {}", style(model).cyan(), desc);
            }

            println!("\n{}", style("Ollama Models:").dim());
            let ollama_models = vec![
                ("llama3.1:8b", "Latest Llama 3.1 (Recommended)"),
                ("llama3:8b", "Llama 3 8B"),
                ("llama3:70b", "Llama 3 70B"),
                ("mistral:7b", "Mistral 7B"),
                ("gemma:2b", "Gemma 2B"),
                ("codellama:7b", "CodeLlama 7B"),
            ];
            for (model, desc) in ollama_models {
                println!("  {} - {}", style(model).cyan(), desc);
            }

            println!("\nCurrent default: {}", style(&config.default_model).yellow().bold());
        }
        ModelCommands::Current => {
            println!("Current default model: {}", style(&config.default_model).yellow().bold());
            println!("Provider: {:?}", config.provider);
        }
        ModelCommands::Use(args) => {
            let mut config = config;
            config.default_model = args.model.clone();
            config.save()?;
            println!("{} Default model set to: {}", style("✓").green().bold(), style(&args.model).yellow().bold());
        }
    }

    Ok(())
}

async fn handle_config_commands(cmd: &ConfigCommands) -> anyhow::Result<()> {
    match cmd {
        ConfigCommands::Show => {
            let config = Config::load();
            println!("Current Configuration:");
            println!("{}", serde_json::to_string_pretty(&config)?);
            println!("\nConfig file: {}", Config::config_file().display());
        }
        ConfigCommands::Export(args) => {
            let config = Config::load();
            config.export_to_file(&args.output)?;
            println!("{} Configuration exported to: {}", style("✓").green().bold(), args.output.display());
        }
        ConfigCommands::Import(args) => {
            let imported_config = Config::import_from_file(&args.input)?;
            let mut config = Config::load();

            if args.merge {
                config.merge(imported_config);
                println!("{} Configuration merged successfully!", style("✓").green().bold());
            } else {
                config = imported_config;
                println!("{} Configuration imported successfully!", style("✓").green().bold());
            }

            config.save()?;
        }
        ConfigCommands::Set(args) => {
            let mut config = Config::load();

            match args.key.to_lowercase().as_str() {
                "default_model" | "model" => {
                    config.default_model = args.value.clone();
                    println!("{} Default model set to: {}", style("✓").green().bold(), args.value);
                }
                "temperature" | "temp" => {
                    if let Ok(t) = args.value.parse::<f32>() {
                        if (0.0..=2.0).contains(&t) {
                            config.temperature = t;
                            println!("{} Temperature set to: {}", style("✓").green().bold(), t);
                        } else {
                            println!("{} Temperature must be between 0.0 and 2.0", style("✗").red().bold());
                            return Ok(());
                        }
                    } else {
                        println!("{} Invalid temperature value", style("✗").red().bold());
                        return Ok(());
                    }
                }
                "max_iterations" | "iterations" => {
                    if let Ok(n) = args.value.parse::<u32>() {
                        if n >= 1 {
                            config.max_iterations = n;
                            println!("{} Max iterations set to: {}", style("✓").green().bold(), n);
                        } else {
                            println!("{} Max iterations must be at least 1", style("✗").red().bold());
                            return Ok(());
                        }
                    } else {
                        println!("{} Invalid max iterations value", style("✗").red().bold());
                        return Ok(());
                    }
                }
                "use_memory" | "memory" => {
                    let value = args.value.to_lowercase();
                    config.use_memory = value == "true" || value == "yes" || value == "y" || value == "1";
                    println!("{} Use memory set to: {}", style("✓").green().bold(), config.use_memory);
                }
                "use_cache" | "cache" => {
                    let value = args.value.to_lowercase();
                    config.use_cache = value == "true" || value == "yes" || value == "y" || value == "1";
                    println!("{} Use cache set to: {}", style("✓").green().bold(), config.use_cache);
                }
                "context_window_size" | "context_window" | "window_size" => {
                    if let Ok(n) = args.value.parse::<usize>() {
                        if n >= 1024 {
                            config.context_window_size = n;
                            println!("{} Context window size set to: {}", style("✓").green().bold(), n);
                        } else {
                            println!("{} Context window size must be at least 1024", style("✗").red().bold());
                            return Ok(());
                        }
                    } else {
                        println!("{} Invalid context window size value", style("✗").red().bold());
                        return Ok(());
                    }
                }
                "provider" => {
                    match args.value.to_lowercase().as_str() {
                        "openai" => {
                            config.provider = ProviderType::OpenAI;
                            println!("{} Provider set to: OpenAI", style("✓").green().bold());
                        }
                        "ollama" => {
                            config.provider = ProviderType::Ollama;
                            println!("{} Provider set to: Ollama", style("✓").green().bold());
                        }
                        _ => {
                            println!("{} Invalid provider. Use 'openai' or 'ollama'", style("✗").red().bold());
                            return Ok(());
                        }
                    }
                }
                _ => {
                    println!("{} Unknown configuration key: {}", style("✗").red().bold(), args.key);
                    println!("\nAvailable keys: default_model, temperature, max_iterations, use_memory, use_cache, context_window_size, provider");
                    return Ok(());
                }
            }

            config.save()?;
        }
        ConfigCommands::Reset => {
            print!("{} Reset configuration to defaults? This cannot be undone. [y/N]: ", style("?").cyan().bold());
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().eq_ignore_ascii_case("y") {
                let config = Config::default();
                config.save()?;
                println!("{} Configuration reset to defaults!", style("✓").green().bold());
            } else {
                println!("Reset cancelled.");
            }
        }
    }

    Ok(())
}

async fn handle_mcp_commands(cmd: &McpCommands) -> anyhow::Result<()> {
    let mut config = Config::load();

    match cmd {
        McpCommands::List => {
            if config.mcp_servers.is_empty() {
                println!("No MCP servers configured.");
                println!("Use `corvus mcp add` to add a server.");
            } else {
                println!("{}", style("Configured MCP Servers:").bold());
                for server in &config.mcp_servers {
                    println!("\n  {}:", style(&server.name).cyan().bold());
                    println!("    Command: {} {}", server.command, server.args.join(" "));
                }
            }
        }
        McpCommands::Add(args) => {
            // Check if server already exists
            if config.mcp_servers.iter().any(|s| s.name == args.name) {
                println!("{} Server '{}' already exists.", style("✗").red().bold(), args.name);
                println!("Use a different name or remove the existing server first.");
                return Ok(());
            }

            config.mcp_servers.push(corvus_cli::config::McpServerConfig {
                name: args.name.clone(),
                command: args.command.clone(),
                args: args.args.clone(),
            });

            config.save()?;
            println!("{} MCP server '{}' added successfully!", style("✓").green().bold(), args.name);
        }
        McpCommands::Remove(args) => {
            let initial_len = config.mcp_servers.len();
            config.mcp_servers.retain(|s| s.name != args.name);

            if config.mcp_servers.len() < initial_len {
                config.save()?;
                println!("{} MCP server '{}' removed successfully!", style("✓").green().bold(), args.name);
            } else {
                println!("{} Server '{}' not found.", style("✗").red().bold(), args.name);
            }
        }
        McpCommands::Test(args) => {
            if let Some(server) = config.mcp_servers.iter().find(|s| s.name == args.name) {
                println!("Testing connection to '{}'...", server.name);

                let mut manager = corvus_cli::mcp_bridge::McpServerManager::new();
                match manager
                    .connect_stdio(
                        server.name.clone(),
                        server.command.clone(),
                        server.args.clone(),
                    )
                    .await
                {
                    Ok(init) => {
                        println!("{} Connected successfully!", style("✓").green().bold());
                        println!("  Server: {} {}", init.server_info.name, init.server_info.version);
                        println!("  Protocol version: {}", init.protocol_version);

                        if let Some(instructions) = &init.instructions {
                            println!("  Instructions: {}", instructions);
                        }

                        // List tools
                        if init.capabilities.tools.is_some() {
                            if let Some(client) = manager.get_client(&server.name) {
                                let mut client = client.lock().await;
                                if let Ok(tools) = client.list_tools().await {
                                    println!("\n  Available tools ({}):", tools.len());
                                    for tool in tools {
                                        println!("    - {}: {}", tool.name, tool.description);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Connection failed: {}", style("✗").red().bold(), e);
                    }
                }
            } else {
                println!("{} Server '{}' not found.", style("✗").red().bold(), args.name);
            }
        }
    }

    Ok(())
}

/// Handle plugin management commands
async fn handle_plugin_commands(cmd: &PluginCommands) -> anyhow::Result<()> {
    match cmd {
        PluginCommands::List => {
            println!("{}", style("Installed Plugins:").bold());
            println!("\n  No plugins installed yet.");
            println!("\n  Use `corvus plugin install <path>` to install a plugin.");
        }
        PluginCommands::Install(args) => {
            println!("{} Installing plugin from: {}", style("⚠").yellow(), args.path);
            println!("{} Plugin installation coming soon!", style("⚠").yellow());
        }
        PluginCommands::Uninstall(args) => {
            println!("{} Uninstalling plugin: {}", style("⚠").yellow(), args.name);
            println!("{} Plugin uninstallation coming soon!", style("⚠").yellow());
        }
        PluginCommands::Enable(args) => {
            println!("{} Enabling plugin: {}", style("⚠").yellow(), args.name);
            println!("{} Plugin enable/disable coming soon!", style("⚠").yellow());
        }
        PluginCommands::Disable(args) => {
            println!("{} Disabling plugin: {}", style("⚠").yellow(), args.name);
            println!("{} Plugin enable/disable coming soon!", style("⚠").yellow());
        }
    }

    Ok(())
}

/// Handle memory management commands
async fn handle_memory_commands(cmd: &MemoryCommands) -> anyhow::Result<()> {
    let store = TagMemoStore::open_default()?;

    match cmd {
        MemoryCommands::Add(args) => {
            let content = args.content.join(" ");
            let content_type = match args.content_type.to_lowercase().as_str() {
                "code" => corvus_core::memory::ContentType::Code,
                "conversation" => corvus_core::memory::ContentType::Conversation,
                "thought" => corvus_core::memory::ContentType::Thought,
                "dream" => corvus_core::memory::ContentType::Dream,
                _ => corvus_core::memory::ContentType::Text,
            };

            let id = store.add_memory(content, content_type, args.tag.clone()).await?;

            println!("{} Memory added successfully!", style("✓").green().bold());
            println!("  ID: {}", style(&id).dim());
            if !args.tag.is_empty() {
                println!("  Tags: {}", style(args.tag.join(", ")).yellow());
            }
        }
        MemoryCommands::List(args) => {
            let memories = store.list_memories(args.limit).await;

            if memories.is_empty() {
                println!("No memories found.");
                return Ok(());
            }

            println!("{}", style("Recent Memories:").bold());
            for memory in memories {
                let date = memory.last_accessed.format("%Y-%m-%d %H:%M");
                let tags = if memory.tags.is_empty() {
                    "".to_string()
                } else {
                    format!("[{}]", memory.tags.join(", "))
                };

                println!("\n  {} {}", style(&memory.item.id.as_deref().unwrap_or("")).dim(), style(date).cyan());
                if !tags.is_empty() {
                    println!("  {}", style(tags).yellow());
                }
                let content_preview = if memory.item.content.len() > 100 {
                    format!("{}...", &memory.item.content[..100])
                } else {
                    memory.item.content.clone()
                };
                println!("  {}", content_preview);
                println!("  Access count: {}", memory.access_count);
            }
        }
        MemoryCommands::Search(args) => {
            let results = store.search_memories(&args.query, args.limit).await;

            if results.is_empty() {
                println!("No memories found for: {}", args.query);
                return Ok(());
            }

            println!("{}", style("Memory Search Results:").bold());
            println!("Query: {}\n", args.query);

            for memory in results {
                let date = memory.last_accessed.format("%Y-%m-%d %H:%M");
                println!("  {} {}", style(&memory.item.id.as_deref().unwrap_or("")).dim(), style(date).cyan());
                println!("  {}", memory.item.content);
                println!();
            }
        }
        MemoryCommands::Export(args) => {
            let memories = store.list_memories(1000).await;

            let content = serde_json::to_string_pretty(&memories)?;
            std::fs::write(&args.output, content)?;

            println!("{} Memories exported to: {}", style("✓").green().bold(), args.output.display());
        }
        MemoryCommands::Import(args) => {
            let content = std::fs::read_to_string(&args.input)?;
            let memories: Vec<corvus_cli::memory_store::StoredMemory> = serde_json::from_str(&content)?;

            println!("{} Importing {} memories...", style("⚠").yellow(), memories.len());
            println!("{} Memory import coming soon!", style("⚠").yellow());
        }
        MemoryCommands::Delete(args) => {
            if store.delete_memory(&args.memory_id).await {
                println!("{} Memory deleted successfully!", style("✓").green().bold());
            } else {
                println!("{} Memory not found: {}", style("✗").red().bold(), args.memory_id);
            }
        }
    }

    Ok(())
}

/// Build an agent with common configuration
async fn build_agent(
    config: &Config,
    completion_model: Arc<dyn CompletionModel>,
    preamble: &str,
) -> anyhow::Result<(Agent, usize)> {
    let mut agent_builder = Agent::builder()
        .completion_model_arc(completion_model)
        .temperature(config.temperature)
        .max_iterations(config.max_iterations)
        .context_window_size(config.context_window_size)
        .preamble(preamble);

    if config.use_memory {
        let memory = InMemoryMemory::new();
        agent_builder = agent_builder.memory(memory);
    }

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("corvus");

    if let Ok(execute_tool) = ExecuteTool::new() {
        agent_builder = agent_builder.tool(execute_tool);
    }
    agent_builder = agent_builder.tool(FileTool::new(&current_dir));
    agent_builder = agent_builder.tool(ShellTool::new(&current_dir));
    agent_builder = agent_builder.tool(GitTool::new(&current_dir));
    agent_builder = agent_builder.tool(SearchTool::new(&current_dir));
    agent_builder = agent_builder.tool(HttpTool::new());
    agent_builder = agent_builder.tool(SystemTool::new());

    // Load MCP servers
    let mut mcp_manager = corvus_cli::mcp_bridge::McpServerManager::new();
    let mut mcp_tool_count = 0;

    for server_config in &config.mcp_servers {
        match mcp_manager
            .connect_stdio(
                server_config.name.clone(),
                server_config.command.clone(),
                server_config.args.clone(),
            )
            .await
        {
            Ok(_) => {
                println!("{} Connected to MCP server: {}", style("✓").green().bold(), server_config.name);
            }
            Err(e) => {
                println!("{} Failed to connect to MCP server '{}': {}", style("⚠").yellow(), server_config.name, e);
            }
        }
    }

    // Add MCP tools to agent
    if let Ok(mcp_tools) = mcp_manager.all_tools().await {
        mcp_tool_count = mcp_tools.len();
        for tool in mcp_tools {
            agent_builder = agent_builder.tool(tool);
        }
    }

    // Load plugins
    let plugin_manager = PluginManager::new(&current_dir, &config_dir);

    // TODO: Load plugins from disk
    // For now, plugin system is set up but no plugins are loaded by default

    // Add plugin tools to agent
    let plugin_tools = plugin_manager.all_tools();
    let plugin_tool_count = plugin_tools.len();
    for tool in plugin_tools {
        agent_builder = agent_builder.tool(tool);
    }

    let total_tool_count = mcp_tool_count + plugin_tool_count;
    let agent = agent_builder.build()?;
    Ok((agent, total_tool_count))
}

async fn run_chat_mode(
    args: corvus_cli::cli::ChatArgs,
    config: Config,
    completion_model: Arc<dyn CompletionModel>,
) -> anyhow::Result<()> {
    println!("Welcome to Corvus Chat! (type 'exit' to quit)\n");

    let model_name = args.model.unwrap_or(config.default_model.clone());

    // Set up chat memory if enabled
    let mut chat_memory = if config.use_memory {
        match ChatMemory::open_default(config.memory_threshold, config.max_memories) {
            Ok(mut memory) => {
                // Load existing memories
                if let Err(e) = memory.load_memories().await {
                    println!("Warning: Could not load existing memories: {}", e);
                }
                println!("✓ Memory system enabled (threshold: {}, max: {})", config.memory_threshold, config.max_memories);
                Some(memory)
            }
            Err(e) => {
                println!("Warning: Could not initialize memory system: {}", e);
                None
            }
        }
    } else {
        None
    };

    let preamble = "You are Corvus, an intelligent AI assistant. You help with coding tasks and general questions. You have access to these tools: execute_code (run code), file_operations (read/write files), shell_exec (run shell commands), git_operations (git commands), file_search (search for files), http_request (make HTTP requests), and system_info (get system information).";
    let (agent, mcp_tool_count) = build_agent(&config, completion_model, preamble).await?;

    println!("\nUsing model: {}", model_name);
    if mcp_tool_count > 0 {
        println!("MCP tools loaded: {}", mcp_tool_count);
    }
    println!();

    let mut storage = SessionStorage::open_default()?;
    let session = storage.create_session(None)?;

    if !args.prompt.is_empty() {
        let prompt = args.prompt.join(" ");
        println!("You: {}", prompt);
        storage.add_message(&session.id, "user", &prompt)?;

        // Save user message to memory and inject memories
        let enhanced_prompt = if let Some(memory) = &mut chat_memory {
            memory.save_user_message(&prompt).await?;
            memory.inject_memories(&prompt).await?
        } else {
            prompt.clone()
        };

        match agent.run(&enhanced_prompt).await {
            Ok(response) => {
                println!("\nCorvus: {}\n", response);
                storage.add_message(&session.id, "assistant", &response)?;

                // Save assistant response to memory
                if let Some(memory) = &mut chat_memory {
                    memory.save_assistant_message(&response).await?;
                }
            }
            Err(e) => print_error(&e.into()),
        }
    }

    let chat_memory_clone = chat_memory.clone();
    run_chat_interactive(agent, storage, &session.id, chat_memory).await?;

    if let Some(memory) = chat_memory_clone {
        let stats = memory.stats();
        println!("\nMemory stats: {} saved, {} retrieved", stats.total_saved, stats.total_retrieved);
    }

    println!("Goodbye! (Session saved: {})", session.id);
    Ok(())
}

async fn run_single_task(
    args: corvus_cli::cli::RunArgs,
    config: Config,
    completion_model: Arc<dyn CompletionModel>,
) -> anyhow::Result<()> {
    let task = if !args.task.is_empty() {
        args.task.join(" ")
    } else {
        return Err(corvus_core::error::CorvusError::InvalidArgument(
            "No task specified".to_string(),
        )
        .into());
    };

    let model_name = args.model.unwrap_or(config.default_model.clone());

    let preamble = "You are Corvus, an intelligent AI assistant. Complete the task thoroughly.";
    let (agent, mcp_tool_count) = build_agent(&config, completion_model, preamble).await?;

    println!("Running task: {}", task);
    println!("Using model: {}", model_name);
    if mcp_tool_count > 0 {
        println!("MCP tools loaded: {}", mcp_tool_count);
    }
    println!();

    match agent.run(&task).await {
        Ok(response) => {
            println!("{}", style("Result:").bold().underlined());
            format_response(&response);
        }
        Err(e) => print_error(&e.into()),
    }

    Ok(())
}

/// Wrap a completion model with caching if enabled
fn wrap_with_cache(
    model: Arc<dyn CompletionModel>,
    config: &Config,
) -> Arc<dyn CompletionModel> {
    if config.use_cache {
        Arc::new(CachedCompletionModel::with_default_cache(model, true))
    } else {
        model
    }
}

/// Create a completion model based on the configuration
async fn create_completion_model(
    config: &Config,
) -> anyhow::Result<Arc<dyn CompletionModel>> {
    let model = match config.provider {
        ProviderType::OpenAI => {
            let api_key = if let Some(key) = &config.openai_api_key {
                key.clone()
            } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                key
            } else {
                return Err(corvus_core::error::CorvusError::Config(
                    "OPENAI_API_KEY environment variable not set".to_string(),
                )
                .into());
            };

            let client = Arc::new(OpenAIClient::new(api_key));
            let model = OpenAICompletionModel::new(client, config.default_model.clone());
            Arc::new(model)
        }
        ProviderType::Ollama => {
            let base_url = if let Some(url) = &config.ollama_base_url {
                url.clone()
            } else {
                "http://localhost:11434/v1".to_string()
            };

            let client = Arc::new(ollama::create_client(base_url));
            let model = ollama::create_completion_model(client, config.default_model.clone());
            Arc::new(model)
        }
    };

    Ok(wrap_with_cache(model, config))
}
