# Feature: Workspace & Config Management

## Goals
- 为不同工程项目建立标准化的本地操作闭环与环境隔离环境 (`src/workspace`)。
- 在用户全局目录 (`~/.zcode/zcode.json`) 以及项目局部 (`.zcode/config.toml`) 设计分层级的配置数据管理树，支持属性向下继承与覆盖。
- 提供基于当前工作区的结构化描述上下文探测器封装（以便向核心智能组件透明化工程细节）。

## Non-Goals
- 全局动态修改其它项目的数据。
- 提供在线远程 Workspace 状态同步（暂处于离线化、无中心服务纯本地运行体系）。

## User Stories
- 作为开发者，我希望我的 API Key 以及全局惯用的大模型（Model）配置保存在 `~/.zcode/zcode.json`，各个项目默认继承这个安全主键。
- 作为某个专有项目的负责人，我希望能在这个特定工程内加入特定的 `<工程根目录>/.zcode/`，为其制定专属加载的 MCP 列表和本地 Skill 等环境数据。
- 作为大语言模型，系统能在启动时将当前项目的代码主轴架构通过 Workspace 层级快速抽取并合并，给我最精准的目录大纲环境。

## System Architecture Context
配置环境作为全局静态层级。
- `src/config/settings.rs`：利用 `serde_json` 获取全局的用户级别环境数据，定义如 `settings.llm`, `settings.mcp_servers`, `settings.skill_dirs` 实体。并暴露出 `Settings::merge` 进行动态参数组合。
- `src/workspace/mod.rs`：通过 `Workspace::open(&cwd)` 抽象进行工程启动。如果包含 Git 工程，它还能代理至 `src/git` 中取得 `diff_context`，并承接起 snapshot 路径控制职责。对于 AI Agent 而言，Workspace 隔离出了一套只属于当前目录安全域的所有状态读写封装。

## Acceptance Criteria
- [ ] 当项目目录没有 `.zcode/` 定义时，可以降级无缝回退到全局定义的 JSON 文件进行加载。
- [ ] `Settings::merge` 方法必须正确且稳健地支持从全局与局部中拼接 Vec 对象（例如全局配置的 MCP 服务应与当前配置的 MCP 服务一起生效）。
- [ ] 支持无缝地实现 JSON -> Config 结构体的序列化互转。相关的测试不能出错。
- [ ] Workspace 载入过程必须优雅处理文件权限、目录不可读等系统 I/O 级异常，向 CLI 抛出对用户友好的人话。
