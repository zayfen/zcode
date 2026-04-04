# Feature: Unified Tooling with MCP & LSP Integration

## Goals
- 为本地与远程的开发原子能力提供统一对外的 `ToolRegistry` 管理与查询调用注册表化能力屏蔽。
- 支持 Model Context Protocol (MCP) 标准，让 zcode 能挂接任何支持 MCP 的社区标准客户端能力，从而大幅补充大模型在本地端操作的能力断档。
- 实现与现有的 LSP (Language Server Protocol) 工具链的高级互动（Hover, 引用检测、语法定义抽取等）。

## Non-Goals
- 不提供直接的大模型预训练调参，仅关注通过工具外挂将工程的“眼与手”延展。
- 不提供 LSP 服务端进程——只作为客户端请求获取现有工程信息。

## User Stories
- 由于我正在对接庞大的企业数据库生态，我很希望能在 `~/.zcode/zcode.json` 装上一个 `sqlite-mcp-server`，并让 zcode 具备读取表的超级能力。
- 此工具需如同本地的底层方法一样可接受参数和结构体，Agent 在需要的时候甚至感觉不到 MCP 服务来自于远程端口还是本地文件系统调换。
- 在发生复杂的重构意图时，我希望 Agent 会熟练使用 LSP 能力查询 `findAllReferences` 获取依赖矩阵情况再动手，这样就可以避免代码拆崩。

## System Architecture Context
利用了高度标准化的接口桥接模式：
- `Tool (trait)`：定义了统一接口形式：`fn execute(&self, input: Value) -> ToolResult<Value>`。
- `ToolsRegistry`：所有的内建工具（`FileTool`, `SearchTool`, `ShellTool` 等）在启动时动态挂接到表单字典内形成清单，在 `llm` 获取指令下达时将自动查表进行调度并返回状态。
- `McpToolAdapter`：极度关键的一个薄封装包装！它实现了与远端 Stdio 子进程的数据双向收发，并在外围套了一层 `Tool trait`，让其可以无缝混进 Registry！

## Acceptance Criteria
- [ ] 确保 `McpClient` 能够可靠地利用标准 JSON-RPC 2.0 规格通过 stdio 呼叫出 `tools/list` 并反手创建适配器扔到注册表。
- [ ] MCP 服务的启停应当接受完整的生命周期管理（由于它是跑独立的子进程），必须防止内存泄漏以及出现僵尸进程。
- [ ] AST、本地文件检索、壳体命令下达 (`ShellTool`) 等核心默认原生工具，不可发生因安全权限缺失而引起直接宕机，仅可捕获异常并将包含友好的文本输出反馈回环送给 LLM 回退判定。
