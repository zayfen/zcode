# Feature: Session Recording & Snapshots

## Goals
- 实现高度稳定可靠的人机对话记录存储与交互全链路历史检索查阅方案。
- 为断电重启、关闭回旋提供工作区现场保护与利用 `Sqlite` 实现极小成本增量状态保存（快照 Snapshot 管理）。
- 将运行完毕的特定大型项目成果与 Task List 以格式化标准输出到 `docs/` 下成为结构化的 Markdown 笔记供跨工程翻阅复查。

## Non-Goals
- 同步上传用户隐私数据对话留存到云厂商服务器（全部贯彻完全 100% 的 Local Offline 持久化）。
- 每毫秒无脑写入记录造成大规模 IO 磨损（要求带有缓冲提交的批控）。

## User Stories
- 最近一次 zcode 给我的几百行精妙的调整代码我没有马上确认，在意外关闭终端界面后我崩溃了！如果它有 Session Record 保存机制，我能够找回我所有的历史思路那简直太重要了！
- 在大规模架构重构作业中，我希望每次 Coder Agent 从 Planner 那里拉到长工单干完一段成果时，能够打快照存档 `snapshot_save`，以便我在发觉重构线路歪曲时可以通过命令自由地向不同历史节点撤回现场覆灭代码错误。

## System Architecture Context
存储底座驱动持久化技术：
- `src/session`（结合 `Sqlite` 依赖）：包含一套精简的基于 `rusqlite` 的状态连接桥接封装用于无感化追踪记录当前系统与模型互动间全量的 Timeline 文本段落信息节点（涵盖 Role 分别标识与文本串）。
- `src/task_store`：与工程结构深度锁定的归档封存逻辑。负责当整个 `Orchestrator` 系统宣布彻底打通全链路（或抛出关键进展）后拉去序列化导出，建立类似于 `.zcode/tasks/xxxx.md` 或在工程工作根目录下暴露 `walkthrough.md` 的可查阅沉淀成果汇报单。

## Acceptance Criteria
- [ ] 建立正确的 SQLite 数据库初始化与表的无干预无感升级建表模式（Migration）。
- [ ] Session 库的数据查询支持按照 `Timestamps` 和工作区目录索引反向精准查找过去时间段内的特定对话流水。
- [ ] 各种保存请求与打快照的方法（由于可能耗时且带阻塞 IO），需要被调度在安全域，不得堵塞 `MessageBus` 引擎总线通信流的正常高并发运转。
- [ ] 确保落库的 Sqlite 敏感数据可以被简单直接的命令实现清理或者定期修剪以抑制硬盘体积在巨量历史中的无限扩张。
