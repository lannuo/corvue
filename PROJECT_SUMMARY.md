# Corvus - 项目总结

## 概述

**Corvus** 是一个结合了 5 个优秀项目优点的 AI Agent CLI 工具：

| 项目 | 借鉴的核心特性 |
|------|----------------|
| **Zed** | 纯 Rust、高性能、模块化设计 |
| **Goose** | 代码执行能力、MCP/ACP 协议、安全沙箱 |
| **Codex** | OpenAI 官方集成、沙箱安全 |
| **Rig** | 多 LLM 提供商抽象、trait 架构、向量存储集成 |
| **VCPToolBox** | 认知记忆系统 (TagMemo)、时间感知、主观能动性 |

## 已完成的工作

### Phase 1: 核心基础 ✅

- **corvus-core** - 完整的 trait 架构
  - `CompletionModel` - 统一的 LLM 完成接口
  - `EmbeddingModel` - 文本嵌入生成
  - `MemorySystem` - 认知记忆接口
  - `Tool` - 工具调用接口
  - `Agent` - 主协调器和代理循环

### Phase 2: LLM 提供商 ✅

- **corvus-providers** - OpenAI 实现
  - GPT-4o、GPT-4o Mini 等的聊天完成
  - text-embedding-3 模型的嵌入
  - 工具调用支持

### Phase 3: 记忆系统 ✅

- **corvus-memory** - 简单内存存储
  - 存储和检索记忆项
  - 基于标签的过滤
  - 语义搜索（基于嵌入）
  - TagMemo V7 认知架构的基础

### Phase 4: CLI 界面 ✅

- **corvus-cli** - 纯 CLI 体验
  - `corvus chat` - 交互式聊天模式
  - `corvus run` - 单任务执行
  - `corvus memory` - 记忆管理
  - `corvus config` - 配置管理

## 项目结构

```
corvus/
├── Cargo.toml                    # 工作区配置
├── README.md                     # 项目说明
├── PROJECT_SUMMARY.md            # 本文档
└── crates/
    ├── corvus-core/              # 核心 traits 和类型
    ├── corvus-providers/         # 多提供商 LLM 支持
    ├── corvus-memory/            # 认知记忆系统
    ├── corvus-execution/         # 代码执行沙箱 (占位)
    ├── corvus-protocol/          # MCP/ACP 协议 (占位)
    ├── corvus-cli/               # 命令行界面
    └── corvus-tools/             # 内置工具 (占位)
```

## 使用方法

### 配置

设置 OpenAI API 密钥：

```bash
export OPENAI_API_KEY="your-api-key-here"
```

或使用配置命令：

```bash
corvus config set openai_api_key your-api-key-here
```

### 使用

**交互式聊天模式：**

```bash
corvus chat
```

**运行单个任务：**

```bash
corvus run "Explain what Rust ownership is"
```

**记忆管理：**

```bash
corvus memory search "Rust"
corvus memory list
```

## 待开发功能

### Phase 5: TagMemo V7 认知记忆 (Coming Soon)

- EPA 模块 - 嵌入投影分析
- 残差金字塔 - Gram-Schmidt 正交分解
- TagMemo Wave 算法 - N-hop 脉冲传播
- USearch 向量索引集成
- SQLite 持久化存储

### Phase 6: 代码执行沙箱 (Coming Soon)

- 跨平台沙箱化代码执行
- Linux: namespaces + seccomp
- macOS: sandbox_init
- Windows: AppContainer
- 多语言支持

### Phase 7: MCP/ACP 协议 (Coming Soon)

- MCP (Model Context Protocol) 服务器/客户端
- ACP/SACP (Agent Client Protocol) 实现
- 内置 MCP 扩展

### Phase 8: 更多 LLM 提供商 (Coming Soon)

- Anthropic Claude
- Google Gemini
- Ollama
- 20+ 其他提供商

### Phase 9: 内置工具 (Coming Soon)

- 文件操作
- Shell 命令
- Git 集成
- 等等

## 设计理念

Corvus 的设计哲学：

1. **纯 Rust** - 高性能、内存安全
2. **Trait 驱动** - 灵活、可扩展的抽象
3. **模块化** - 清晰的 crate 边界
4. **认知优先** - TagMemo 提供类似人类的记忆
5. **开发者工具** - 专注于个人开发者体验

## 技术栈

- **异步运行时**: tokio
- **序列化**: serde, serde_json
- **CLI 框架**: clap, cliclack
- **数据库**: rusqlite, tokio-rusqlite
- **向量数学**: ndarray, linfa
- **HTTP 客户端**: reqwest
- **追踪**: tracing, tracing-subscriber

## License

MIT OR Apache-2.0
