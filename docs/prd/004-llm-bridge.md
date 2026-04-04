# Feature: Multi-Provider LLM Bridge

## Goals
- 提供基于 `LlmProvider` 大一统接口隔离机制的大型语言模型底座适配层。
- 支持对接市场最高水准的多流派接口格式（如提供 Anthropic/Claude3 的原生 XML 或者 SSE 长接入、以及 OpenAI 的 Function Calling）。
- 提供私密轻量级开源本地模型降级支持方案 (由 `Ollama` 提供环境支持并桥接)。

## Non-Goals
- 直接针对每种模型格式重新撰写冗余的外包装结构图；要求核心领域层只面对标准包装好的统一类型结构交互。
- 搭建专门的大模型网关——我们仅作为 Consumer 使用。

## User Stories
- 作为不同厂商信仰粉，由于有些大秘钥过于昂贵，我期望能够同时使用公司配发的 `Claude-3.5` API 来写复杂逻辑应用代码，用本地的 `Ollama Llama-3` 小模型来进行轻量级的日志归纳或单元测试撰改。
- 系统必须能够直接消化大模型反馈的长字符串且自行拦截并分析哪些是 Tool 触发，将这一工作透明封装化，省去复杂的流控制台解析。

## System Architecture Context
模块为极度典型的适配器（Adapter）与防腐层。
- `LlmProvider (Trait)`：所有对接库的底座标准。主要存在 `chat(history: &[Message], tools: &[ToolCallSpec]) -> Result<LlmResponse>` 标准方法支持。
- `LlmConfig`：用于配置注入参数与身份签名。包含 `model`、`temperature` 的通用化设定载体。
- `RigProvider` 等实现层依据内部定义，利用 `reqwest` 或第三方绑定引擎真正下发 HTTP 请求。底层解析时，会将 Claude 等特殊的 Tools 返回封装成通用的统一指令实体供外层图（Graph Loop）流转调用。
  
## Acceptance Criteria
- [ ] `LlmProvider::chat` 必须能严密地支持传入和接收 `ToolCallSpecs` json 结构化定义规范，兼容目前主线厂商能力。
- [ ] 面向不同服务商的 HTTP 调用失败（403 Auth error, 429 Rate Limit）时必须抛出一个格式化良好，带有明确指纹的 `ZcodeError` 结构到宿主环境。
- [ ] 包含基于超长 `tokens` 输出的截断自动控制阈值机制（`max_tokens` 参数有效落实）。
- [ ] 系统级别对于底层敏感秘钥 (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) 等数据不得在日志与回溯堆栈里被泄露出来。
