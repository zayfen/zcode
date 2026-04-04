# Feature: Multi-Language Scripting & Skills

## Goals
- 为进阶开发者赋予使用熟悉的非 Rust 动态开发语言 (Lua, JavaScript, Python 等) 自定义增强内部控制链与功能逻辑的平台支持能力。
- 将动态外围脚本利用 `Skills` 仓库系统化包统一化加载管理，供底层智能图 Agent 作为可调度的超强外挂能力来执行非原生功能的扩展延伸。
- 为这些引擎环境动态暴露注入一份 Zcode 本源基础功能对象 API。

## Non-Goals
- 全方位兼容与接管系统级别的复杂网络底层应用接口开发。限定脚本仅仅作为轻量级别钩子（Cookbook / Hook）。
- 让外部非安全脚本在提权的情况下随意获取到敏感宿主权限。

## User Stories
- 由于我发现大语言模型的原生抓取能力在面对复杂的登录 Token 二次签发流程时常常懵逼无助，我作为一个研发，能够随时写一个 `JS` 外挂技能丢在 `.zcode/skills` 中让语言模型进行远程动态回调使用，大幅减轻其思维压力的同时也增加了复用性！
- 当我在编写规范的时候，我想借助 `LuaEngine` 或者 `PythonEngine` 开发前置编译钩子进行静态代码的格式预检测屏蔽不合格代码！

## System Architecture Context
这是一个多端集成嵌入式运行时模块沙盘域。
- `ScriptEngine (Trait)`: 标准统一桥接特质：约束了 `eval` 和 `call_function`、 `handles` 及扩展特征探测的能力。
- 各大派系实现实现载体与依赖挂载点：
  - `LuaEngine` -> 使用 `mlua` 提供服务支撑。
  - `PythonEngine` -> 以 `pyo3` 打造原生的双向 C-Rust 调用结构。
  - `JsEngine` -> 引入轻量并强劲的 `rquickjs` 解析执行底层。
- `ScriptManager` 负责充当统领总督，遍历本地策略或全局挂靠的 `skill_dirs`，获取可接纳的脚本，并执行热拔插 `ScriptTool` 的封装代理过程并一并塞入 `ToolsRegistry` 注册。

## Acceptance Criteria
- [ ] 系统支持能够顺利捕获所有的脚本语法崩溃及抛出未授权异常指令不至于让主流程受到牵连宕机。
- [ ] 所有引擎能被正常加载且无缝为其中的脚本动态引入系统所注入的保留基础控制宏能力（包含如：`read_file`, `write_file`, `shell` 执行和专有 `log` 对象），并保障回调回包无数据解析污染。
- [ ] 当挂载一个不存在其语言运行时（例如当前宿主环境没有对应的完整 python 环境包而引入了需要 py 通信）需要优雅向用户提示缺少对应语言底层依赖支撑错误。
