# Corvus 发布检查清单

## 预发布检查

### 代码质量
- [x] 所有单元测试通过 (35/35)
- [x] 项目可以成功构建
- [x] 主要警告已清理
- [x] 示例代码已创建
- [ ] 集成测试已创建并通过

### 文档
- [x] README.md 已更新
- [x] PROJECT_SUMMARY.md 存在
- [x] PERFORMANCE.md 已创建
- [x] USER_EXPERIENCE.md 已创建
- [x] RELEASE_CHECKLIST.md 已创建
- [x] API 文档已生成 (`cargo doc`)
- [ ] 示例文档已完成

### 功能验证
- [x] TagMemo V7 认知记忆
  - [x] EPA 模块
  - [x] Residual Pyramid
  - [x] TagMemo Wave
  - [x] SQLite 存储
- [x] 代码执行沙箱
  - [x] 跨平台支持
  - [x] 语言检测
- [x] 内置工具
  - [x] FileTool
  - [x] ShellTool
  - [x] GitTool
  - [x] ExecuteTool
- [x] MCP/ACP 协议
  - [x] 协议类型
  - [x] 客户端实现
  - [x] 传输层
- [x] 多提供商 LLM
  - [x] OpenAI 支持

## 发布步骤

### 1. 最终测试
```bash
# 运行所有测试
cargo test

# 运行 clippy 检查
cargo clippy

# 构建发布版本
cargo build --release
```

### 2. 生成文档
```bash
# 生成 API 文档
cargo doc --no-deps --open

# 检查文档是否完整
```

### 3. 版本更新
- [ ] 更新 `Cargo.toml` 中的版本号
- [ ] 更新 `README.md` 中的版本信息
- [ ] 创建 git tag (如果适用)
- [ ] 编写发布说明

### 4. 发布到 crates.io (可选)
```bash
# 登录 crates.io
cargo login

# 按顺序发布 crates
cargo publish -p corvus-core
cargo publish -p corvus-providers
cargo publish -p corvus-memory
cargo publish -p corvus-execution
cargo publish -p corvus-protocol
cargo publish -p corvus-tools
cargo publish -p corvus-cli
```

## 已知限制

### 当前限制
1. 仅支持 OpenAI 提供商
2. 会话历史未持久化
3. 缺少配置向导
4. 集成测试尚未完善
5. MCP 客户端需要实际测试

### 未来改进
- [ ] 添加更多 LLM 提供商 (Anthropic, Google, Ollama)
- [ ] 实现会话历史持久化
- [ ] 添加交互式配置向导
- [ ] 完善集成测试
- [ ] 真实场景测试
- [ ] 性能基准测试
- [ ] OpenTelemetry 集成
- [ ] Web 界面原型

## 当前状态

| 指标 | 状态 |
|------|------|
| 单元测试 | 35/35 通过 ✅ |
| 构建状态 | 成功 ✅ |
| 核心功能 | 100% 完成 ✅ |
| 示例代码 | 2 个完整示例 ✅ |
| 文档 | 完整计划 ✅ |

## 快速开始

### 构建项目
```bash
cd corvus
cargo build --release
```

### 运行示例
```bash
# 基本工作流示例
cargo run --example basic_workflow

# MCP 服务器示例
cargo run --example simple_mcp_server
```

### 使用 CLI
```bash
# 查看帮助
cargo run -- --help

# 交互式聊天
cargo run -- chat

# 运行单任务
cargo run -- run "Your task here"

# 查看配置
cargo run -- config
```

## 下一步优先级

### 高优先级
1. ✅ 交互式配置向导 (`corvus setup`)
2. ✅ 更好的错误提示和解决建议
3. ✅ 会话历史持久化

### 中优先级
4. 输出格式化和颜色编码
5. Tab 补全和命令历史
6. 进度指示器

### 低优先级
7. Markdown 渲染支持
8. 向量搜索优化
9. Web 界面原型
