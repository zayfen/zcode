# Feature: Context & Memory Management

## Goals
- 使 Zcode 突破由于 LLM 原生有限 Token Context 窗口导致的短视遗忘症。
- 实现高效的动态切片、合并抽取组合器 (`ContextAssembler`) 管理投递到大语言模型的精准记忆片区。
- 采用向量特征 (`SemanticIndex`) , 基于强语义检索出过往的历史工程片段从而保证 AI 在庞大的工作区迷宫中保持上下文敏锐感知。
- 提供原生的 `GitDiff` 获取以追踪上下文关联环境修改。

## Non-Goals
- 搭建庞大的类似 Elasticsearch 倒排网络检索系统。本地轻量与矢量近似优先。
- 在云上执行向量化的复杂分析（目前全量保持基于本地 Sqlite / File-based 资源分析）。

## User Stories
- 作为项目核心骨干研发：在面对十万行级别以上的新工程入库时，直接扔所有文件给 LLM 会被立刻拦截；我期望内存机制能通过提取 `AST` 和局部 Git 更改，聪明地“自动组合并剪短”能讲清前因后果的代码块上下文喂给 Coder Agent。
- 在与 Zcode 长时间连续聊天后，就算我谈起昨天讨论的“登陆验证逻辑的修改”，它可以像拥有短程记忆力那般精准从上下文中取回上文并衔接问答。

## System Architecture Context
多级混合存储和汇编记忆池架构：
- `WorkingMemory`：高速的进程内部级缓存层。储存热数据的 K/V 对和本次指令环上的会话历程（Session History），通常用其组装出第一顺位的临时 prompt。
- `ProjectMemory`：长期的底层存储逻辑映射。负责关联和留存在工程特定 `.zcode` 目录中的 markdown 或者重要笔记描述文件。
- `SemanticIndex`：专用的 Vector 内存嵌回检索引擎，为了精准匹配大型需求时的相关文档碎片。
- `ContextAssembler`：**控制神经核心！** 所有不同级别 Memory 所搜刮到的文件内容都通过 Assembler 进入并在此进行截断，这通过 `TokenBudget` 参数调控对超出大模型限额的部分进行降段剔除或摘要（Summarize）。

## Acceptance Criteria
- [ ] 当给定的原始代码总文字数据（AST+Diff）远远超越了指定的模型限制 `max_tokens`（如超过 12,000） 时，汇编器 (`ContextAssembler`) 将必须正确裁剪内容防止 400 Bad Request。
- [ ] `GitDiff` Context 构建器不能依赖厚重的 libgit2 二进制库环境，需采用标准的 Git Subprocess 原语调用达到轻量化获取跨文件快照变更的目标。
- [ ] 确保 SemanticIndex 中缓存和运算矢量不会极度拖慢或者卡死整个 TUI 与 Agent 的响应主进程。
