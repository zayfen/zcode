# Feature: Agent Orchestration Engine

## Goals
- 实现一套符合 LangGraph 理念的模块化系统路由与任务下发引擎（由 `BusHandle` 驱动）。
- 构建 `Orchestrator`, `Planner`, `Coder` 和 `Reviewer` 的内部任务流水线，将不同的系统请求按职能透明路由，从而使得思考质量有阶段上的提纯。
- 提供针对所有角色标准化的 `AgentTrait` 接口，以便实现高度可扩展和插拔式系统。

## Non-Goals
- 单个大模型包揽全工程实现逻辑——我们的诉求是流水线作业。
- 脱离总线直接在不同底层服务间进行错乱的点对点硬耦合 RPC。

## User Stories
- 作为项目请求发起者，由于我的请求“去实现用户权限登录功能！”过于宏大，我需要 Orchestrator 启动分流计算。
- 我需要看到经过分流后，系统能将粗粒度的请求拆解成 `[Planner: 拆分为数据库修改、API添加] -> [Coder: 生成第一步的实现码] -> [Reviewer: 交叉代码安全与逻辑审阅]` 的稳定生命周期。
- 如果代码被判定没通过（存在逻辑 panic 风险或者超长行），流水线引擎能够拒绝执行并自动降级或退回修改区要求下一轮（Loop）。

## System Architecture Context
模块化单体环境下的 MPSC 总线通信矩阵：
- `src/agent/mod.rs`：定义顶级接口 `AgentTrait` 与图构建逻辑。所有 Agent 处于后台线程 `tokio::spawn`。
- Agent Pipeline（图态流转）：
  1. `Orchestrator`：决策任务的执行分支流向。
  2. `Planner`：分析输入流的拆解子计划队列。
  3. `Coder`：代码发生器（携带完整的 Tools Callback 能力）。
  4. `Reviewer`：强依赖静态编译环境以及 LLM 进行 Logic、Safety、Testing 断言把关。
- `src/agent/loop_exec.rs`：单任务级别的 Agent Loop 状态机实现（包含单次请求、Prompt组装、模型投递、Tool回调解析乃至最后 Token 扣算等标准流）。

## Acceptance Criteria
- [ ] MPSC 总线系统在大吞吐（数千次流转）情况下不得发生死锁与协程恐慌。
- [ ] 所有实现了 `AgentTrait` 的组件均应具备标准输入与通过 `Result<GraphState>` 报告输出的能力。
- [ ] 当流水线中某一环（如网络异常的 Planner）崩溃时，整个管道应当正常降级并上报 ZcodeError 兜底，终止流程而不是挂起。
- [ ] 调度控制阀必须将流转次数显式控制在一个硬上限阈值内（如配置的 `max_iterations`），彻底杜绝无意义的 Agent 重复回旋死循环问题。
