# Corvus

An intelligent AI Agent CLI tool combining the best features from Zed, Goose, Codex, Rig, and VCPToolBox.

## Overview

**Corvus** (Latin for crow - intelligent, adaptable, tool-using) is a pure Rust AI Agent designed as a personal developer tool. It combines:

- **Code execution capability** (like Goose)
- **Cognitive memory system** (like VCPToolBox's TagMemo)
- **Multi-provider LLM support** (like Rig)
- **High-performance Rust architecture** (like Zed)

## Project Structure

```
corvus/
├── Cargo.toml                    # Workspace config
├── crates/
│   ├── corvus-core/              # Core traits and types
│   ├── corvus-providers/         # Multi-provider LLM support
│   ├── corvus-memory/            # Cognitive memory system
│   ├── corvus-execution/         # Code execution sandbox
│   ├── corvus-protocol/          # MCP/ACP protocol
│   ├── corvus-cli/               # Command-line interface
│   ├── corvus-tools/             # Built-in tools
│   └── corvus-plugin/            # Plugin system
└── README.md
```

## Features Implemented

### Phase 1: Core Foundation (Completed)

- **corvus-core** - Complete trait architecture
  - `CompletionModel` - Unified LLM completion interface
  - `EmbeddingModel` - Text embedding generation
  - `MemorySystem` - Cognitive memory with TagMemo support
  - `Tool` - Tool calling interface
  - `Agent` - Main orchestrator with agentic loop

### Phase 2: LLM Providers (Completed)

- **corvus-providers** - OpenAI implementation
  - Chat completions with GPT-4o, GPT-4o Mini, etc.
  - Embeddings with text-embedding-3 models
  - Tool calling support

### Phase 3: Memory System (Completed)

- **corvus-memory** - TagMemo V7 cognitive memory
  - EPA (Embedding Projection Analysis) module
  - Residual Pyramid (Gram-Schmidt orthogonal decomposition)
  - TagMemo Wave (N-hop spike propagation, LIF neurons)
  - SQLite persistent storage
  - In-memory storage option

### Phase 4: Code Execution Sandbox (Completed)

- **corvus-execution** - Cross-platform sandboxed code execution
  - Linux namespace sandbox support
  - macOS sandbox_init support
  - Language detection and auto-execution
  - Command blocking and security

### Phase 5: Built-in Tools (Completed)

- **corvus-tools** - 7 essential tools
  - `FileTool` - Read/write/list/delete files and directories
  - `ShellTool` - Execute shell commands
  - `GitTool` - Git operations (status, add, commit, push, pull, etc.)
  - `ExecuteTool` - Sandboxed code execution
  - `SearchTool` - File search with glob pattern matching
  - `HttpTool` - HTTP requests (GET, POST, PUT, DELETE, etc.)
  - `SystemTool` - System information (OS, CPU, etc.)

### Phase 6: MCP/ACP Protocol (Completed)

- **corvus-protocol** - Complete MCP implementation
  - JSON-RPC 2.0 protocol
  - Full MCP method support (initialize, ping, tools/list, tools/call, etc.)
  - Stdio transport for MCP servers
  - High-level MCP client API
  - MCP server framework
  - Configuration support in CLI

### Phase 7: CLI Interface (Completed)

- **corvus-cli** - Pure CLI experience
  - `corvus chat` - Interactive chat mode
  - `corvus run` - Single task execution
  - `corvus memory` - Memory management
  - `corvus config` - Configuration management
  - `corvus session` - Session management (list, continue, show, rename, delete, search, export, import)
  - `corvus model` - Model management (list, current, use)
  - `corvus mcp` - MCP server management (list, add, remove, test)
  - `corvus plugin` - Plugin management (list, install, uninstall, enable, disable) - framework complete

### Additional Features (Completed)

- **Response Caching** - LRU-based response caching with time-based expiration
- **Context Window Management** - Smart context trimming with system message retention
- **Vector Search** - k-NN index with cosine similarity for semantic search
- **Plugin System** - Extensible plugin architecture (WASM support planned)
- **Memory Integration** - Vector search integrated with memory system

## More LLM Providers

- ✅ **OpenAI** - GPT-4o, GPT-4o Mini, GPT-4 Turbo, GPT-3.5 Turbo
- ✅ **Ollama** - Llama 3.1, Llama 3, Mistral, Gemma, CodeLlama, etc.

## Coming Soon

- **Anthropic Provider** - Claude 3 models
- **Google Provider** - Gemini models
- **OpenTelemetry** - Observability and tracing
- **WASM Plugins** - Dynamic plugin loading
- **Agent Dream** - Simulated thinking capabilities
- **More Vector Stores** - USearch, FAISS, Qdrant integration

## Quick Start

### Prerequisites

- Rust 1.70+
- OpenAI API key

### Installation

```bash
cd corvus
cargo build --release
```

### Configuration

Set your OpenAI API key:

```bash
export OPENAI_API_KEY="your-api-key-here"
```

Or use the config command:

```bash
corvus config set openai_api_key your-api-key-here
```

### Usage

**Interactive chat mode:**

```bash
corvus chat
```

**Run a single task:**

```bash
corvus run "Explain what Rust ownership is"
```

**Configuration management:**

```bash
corvus config show
corvus config set default_model gpt-4o
corvus config set use_cache true
```

**Session management:**

```bash
corvus session list
corvus session continue <session-id>
corvus session search "Rust"
```

**Model management:**

```bash
corvus model list
corvus model use gpt-4o-mini
```

**MCP server management:**

```bash
corvus mcp list
corvus mcp add my-server command arg1 arg2
corvus mcp test my-server
```

## Design Philosophy

Corvus combines the best ideas from 5 excellent projects:

| Feature | Inspired By |
|---------|-------------|
| Cognitive memory (TagMemo) | VCPToolBox |
| Code execution sandbox | Goose |
| Multi-provider abstraction | Rig |
| Trait-based architecture | Rig |
| Pure Rust, high performance | Zed |
| OpenTelemetry instrumentation | Rig |

## License

MIT OR Apache-2.0
