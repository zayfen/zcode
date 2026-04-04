# Cognition Engine 设计文档

> **状态**: 草案 (Draft)
> **日期**: 2026-03-29
> **作者**: zcode architecture team
> **关联**: ARCHITECTURE.md, src/memory/, src/workspace/

---

## 1. 背景与动机

### 1.1 现状分析

zcode 当前的记忆系统由四个模块组成（`src/memory/mod.rs:1-17`），存在以下关键限制：

| 模块 | 文件 | 问题 |
|------|------|------|
| `SemanticIndex` | `src/memory/semantic.rs` | 基于 TF-IDF（`semantic.rs:21-87`），仅做词频匹配，无法理解语义。`tokenize()` 函数（`semantic.rs:24-29`）仅做 ASCII 字母分割，对中文、代码标识符（如 `VecDeque`、`HashMap`）效果极差 |
| `WorkingMemory` | `src/memory/working.rs` | 纯内存 `VecDeque`（`working.rs:104-119`），进程结束即丢失，跨会话无法保留上下文 |
| `ProjectMemory` | `src/memory/project.rs` | SQLite 后端已有（`project.rs:49-51`），但仅支持 key-value + category 查询（`project.rs:168-193`），无语义检索能力。`CodeChunk` 表虽有 `embedding_json` 字段（`project.rs:34`）但实际未被使用 |
| `ContextAssembler` | `src/memory/context.rs` | Token 预算管理良好（`context.rs:19-70`），但数据源有限：只能注入 project memory 的 category 搜索 + TF-IDF 结果 + 最近文件（`context.rs:156-255`） |

### 1.2 核心痛点

1. **语义搜索质量差**: TF-IDF 无法处理同义词（"error handling" vs "错误处理" vs `Result<T>`）、代码结构语义（"the function that parses config" 无法命中 `fn load()`）
2. **每次启动从零开始**: `WorkingMemory` 没有持久化机制，每次打开项目丢失所有会话上下文
3. **知识获取被动**: 系统只能等用户提供文件或搜索结果，无法主动规划需要查阅哪些知识源
4. **无外部知识集成**: 缺乏 Web 文档获取、MCP 能力发现、依赖分析等能力

### 1.3 目标

构建 **Cognition Engine**（认知引擎），使 zcode 从"被动检索工具"转变为"主动知识获取与推理系统"。

---

## 2. 设计目标

| # | 目标 | 可度量指标 |
|---|------|-----------|
| G1 | 语义搜索替代 TF-IDF | 查询 "error handling" 能命中 `Result<T>` / `ZcodeError` 相关代码块 |
| G2 | 项目打开时自动索引 | `Workspace::open()` 时后台构建索引，万文件项目 < 30s |
| G3 | 跨会话记忆持久化 | 重启后能恢复上次会话的上下文、学到的模式 |
| G4 | 知识获取管道 | 给定需求，自动规划并执行知识收集 |
| G5 | 离线优先 | 所有核心功能（嵌入计算、索引、检索）无网络依赖 |
| G6 | 与现有模块无缝集成 | 不破坏 `ContextAssembler`、`Workspace`、`ToolRegistry` 现有 API |

---

## 3. 核心抽象

### 3.1 Cognition Engine 顶层 trait

```rust
/// 认知引擎 — 统一的知识获取与推理接口
///
/// 整合向量索引、跨会话记忆、知识管道三大子系统
pub trait CognitionEngine: Send + Sync {
    /// 语义搜索：返回与 query 最相关的 top_k 代码/文档片段
    fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult>;

    /// 索引项目文件（全量或增量）
    fn index_project(&self, root: &Path, mode: IndexMode) -> Result<IndexStats>;

    /// 获取组装好的知识上下文（用于注入 LLM prompt）
    fn assemble_knowledge(&self, query: &str, budget: &TokenBudget) -> KnowledgeContext;

    /// 存储会话记忆
    fn store_session_memory(&self, memory: &SessionMemory) -> Result<()>;

    /// 检索相关历史记忆
    fn recall_memories(&self, query: &str, limit: usize) -> Vec<MemoryRecall>;

    /// 获取外部知识（Web 文档、MCP 能力等）
    fn acquire_knowledge(&self, plan: &KnowledgePlan) -> Result<KnowledgeContext>;
}

#[derive(Debug, Clone)]
pub enum IndexMode {
    /// 全量重建索引
    Full,
    /// 仅索引新增/修改文件
    Incremental { changed_files: Vec<PathBuf> },
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub duration_ms: u64,
}
```

### 3.2 向量嵌入抽象

```rust
/// 文本嵌入模型 trait — 抽象底层推理后端
pub trait EmbeddingModel: Send + Sync {
    /// 模型维度（如 384, 768, 1024）
    fn dimension(&self) -> usize;

    /// 对单个文本计算嵌入向量
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// 批量嵌入（某些后端可利用批处理优化）
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // 默认逐个计算，子类可覆盖
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// 模型标识（用于缓存匹配）
    fn model_id(&self) -> &str;
}
```

### 3.3 知识管道抽象

```rust
/// 知识源 — 可被查询的外部知识提供者
pub trait KnowledgeSource: Send + Sync {
    /// 知识源名称
    fn name(&self) -> &str;

    /// 查询知识源，返回相关文档片段
    fn query(&self, query: &KnowledgeQuery) -> Result<Vec<KnowledgeFragment>>;

    /// 该知识源的相关性评分（0.0-1.0），用于排序
    fn relevance(&self, query: &KnowledgeQuery) -> f32;
}

/// 知识查询请求
#[derive(Debug, Clone)]
pub struct KnowledgeQuery {
    /// 原始用户需求
    pub requirement: String,
    /// 提取的关键概念
    pub concepts: Vec<String>,
    /// 需要的知识类型
    pub kind: KnowledgeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeKind {
    /// API 用法与示例
    ApiUsage,
    /// 架构设计与模式
    Architecture,
    /// 代码示例
    CodeExample,
    /// 依赖库文档
    DependencyDoc,
    /// 项目上下文
    ProjectContext,
}

/// 知识片段
#[derive(Debug, Clone)]
pub struct KnowledgeFragment {
    pub source: String,
    pub content: String,
    pub relevance_score: f32,
    pub kind: KnowledgeKind,
}
```

### 3.4 会话记忆抽象

```rust
/// 跨会话持久化记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemory {
    /// 会话 ID
    pub session_id: String,
    /// 会话摘要
    pub summary: String,
    /// 关键决策记录
    pub decisions: Vec<DecisionRecord>,
    /// 学习到的项目模式
    pub learned_patterns: Vec<LearnedPattern>,
    /// 时间戳
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// 决策描述
    pub description: String,
    /// 决策理由
    pub rationale: String,
    /// 相关文件/模块
    pub context_files: Vec<String>,
    /// 嵌入向量（用于语义检索）
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// 模式名称（如 "naming convention", "error handling pattern"）
    pub name: String,
    /// 模式描述
    pub description: String,
    /// 示例
    pub examples: Vec<String>,
    /// 嵌入向量
    pub embedding: Option<Vec<f32>>,
}

/// 记忆召回结果
#[derive(Debug, Clone)]
pub struct MemoryRecall {
    pub memory_type: MemoryType,
    pub content: String,
    pub relevance_score: f32,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryType {
    SessionSummary,
    Decision,
    LearnedPattern,
    ProjectKnowledge,
}
```

---

## 4. 向量嵌入方案选型

### 4.1 候选方案对比

| 维度 | `fastembed` (ONNX) | `candle` (HuggingFace) | `rust-bert` |
|------|---------------------|------------------------|-------------|
| **底层** | ONNX Runtime (`ort` crate) | 纯 Rust推理 | PyTorch-derived, libtorch FFI |
| **推荐模型** | `all-MiniLM-L6-v2` (384d) | `all-MiniLM-L6-v2` ONNX | BERT 变体 |
| **MSRV** | 需要 `ort` 的 C++ 编译 | 纯 Rust, stable 1.75+ | 需要 libtorch 链接 |
| **编译时间** | 中等（vendored ONNX Runtime ~2-3min） | 较快（纯 Rust） | 慢（libtorch ~500MB） |
| **模型加载** | 自动下载 HF 模型 | 需手动下载 | 自动下载 |
| **离线支持** | 首次需下载，后续缓存 | 需预置模型文件 | 首次需下载 |
| **二进制大小** | ~30MB (含 ONNX Runtime) | ~15MB | ~200MB (含 libtorch) |
| **GPU 支持** | 通过 ONNX Session 可选 | 手动实现 | CUDA 支持 |
| **Rust 生态成熟度** | 高（docs.rs 44 个模型变体） | 高（HF 官方维护） | 中等 |
| **维护状态** | 活跃 (v4.x) | 活跃 | 维护模式 |

### 4.2 推荐: `fastembed` (带备选策略)

**推荐使用 `fastembed` crate**，理由：

1. **开箱即用**: 内置 44 种嵌入模型变体（`EmbeddingModel` enum），包含 `AllMiniLML6V2`（384维，平衡精度与速度）
2. **自动下载+缓存**: 首次运行自动从 HuggingFace 下载模型，后续使用本地缓存
3. **满足离线要求**: 模型缓存后完全离线运行
4. **API 简洁**: `TextEmbedding::try_new()` + `embed()` 即可
5. **二进制可接受**: ONNX Runtime vendored 增加约 30MB，对 CLI 工具可接受

**备选策略**: 通过 `EmbeddingModel` trait 抽象，如果 `fastembed` 编译问题过大，可切换到：
- **Phase A**: 纯 Rust 的 `candle` 后端（手动加载 ONNX 模型）
- **Phase B**: 简单的 TF-IDF 增强版（作为零依赖 fallback）

### 4.3 嵌入模型选择

| 模型 | 维度 | 速度 | 质量 | 推荐场景 |
|------|------|------|------|----------|
| `AllMiniLML6V2` | 384 | 快 | 好 | **默认选择**，代码搜索 |
| `AllMiniLML12V2` | 384 | 中 | 很好 | 需要更高质量 |
| `BGEBaseENV15` | 768 | 慢 | 最佳 | 大型项目 / 多语言 |
| `BGESmallENV15` | 384 | 快 | 好 | 低资源环境 |

**默认**: `AllMiniLML6V2` — 384 维向量，80MB 模型，单条嵌入 ~2ms (CPU)。

### 4.4 依赖变更

```toml
# Cargo.toml 新增
[dependencies]
# 向量嵌入
fastembed = "4"

# SQLite 向量搜索（可选，见存储设计）
# sqlite-vec = "0.1"  # 需 MSRV 1.86+，暂不使用
```

---

## 5. 项目索引架构

### 5.1 文件分块策略 (Chunking)

```rust
/// 代码分块器 — 将源文件拆分为可嵌入的片段
pub struct CodeChunker {
    /// 最大块大小（字符数）
    max_chunk_size: usize,
    /// 重叠大小
    overlap_size: usize,
}

/// 代码块
#[derive(Debug, Clone)]
pub struct CodeBlock {
    /// 文件路径
    pub path: PathBuf,
    /// 起始行号
    pub start_line: usize,
    /// 结束行号
    pub end_line: usize,
    /// 块内容
    pub content: String,
    /// 块类型
    pub kind: BlockKind,
    /// 语义标识符（函数名、类名等）
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Function,
    Method,
    Struct,
    Trait,
    Impl,
    Module,
    Test,
    Config,
    DocComment,
    Markdown,
    Unknown,
}
```

**分块规则**:

1. **AST 感知分块**: 利用现有 `tree-sitter`（`src/ast/parser.rs`）解析源文件，按 AST 节点（函数、类、trait、impl）切分
2. **Markdown 分块**: 按 `##` / `###` 标题切分
3. **配置文件**: 整文件作为单个块
4. **大函数处理**: 超过 `max_chunk_size`（默认 1500 字符）的函数按行切分，带 `overlap_size`（默认 100 字符）重叠
5. **元数据增强**: 每个块附带 `path:start_line-end_line` 作为 ID，以及标识符名称

### 5.2 索引流程

```
Workspace::open(project_root)
    │
    ▼
CognitionEngine::index_project(root, IndexMode::Full)
    │
    ├── 1. 文件发现 (walkdir, 尊重 .gitignore)
    │   └── 过滤: 仅索引源码/配置/文档文件
    │
    ├── 2. 文件分块 (并行)
    │   ├── AST 解析 (tree-sitter)
    │   ├── 按函数/类/trait/模块切分
    │   └── 为每个块生成 CodeBlock
    │
    ├── 3. 嵌入计算 (并行批处理)
    │   ├── embed_batch(chunks, batch_size=32)
    │   └── 进度回调: "已索引 128/1024 文件"
    │
    ├── 4. 存储到 SQLite
    │   ├── INSERT code_blocks + embeddings
    │   └── UPDATE file_hashes (用于增量索引)
    │
    └── 5. 返回 IndexStats
```

### 5.3 增量索引

```rust
/// 增量索引管理器
pub struct IncrementalIndexer {
    /// 文件内容哈希缓存 path → blake3_hash
    file_hashes: HashMap<PathBuf, String>,
}

impl IncrementalIndexer {
    /// 检测变更文件
    pub fn detect_changes(&self, root: &Path) -> Vec<FileChange> {
        // 1. 遍历文件，计算 blake3 hash
        // 2. 对比存储的 hash
        // 3. 返回 Added / Modified / Deleted 列表
    }
}

#[derive(Debug, Clone)]
pub enum FileChange {
    Added(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}
```

**增量策略**: 每次 `Workspace::open()` 时:
1. 加载 `file_hashes` 表（存储在 SQLite）
2. 快速扫描文件列表，对修改时间变化的文件计算 hash
3. 仅对 `Added` / `Modified` 文件重新分块+嵌入
4. 删除 `Deleted` 文件的旧块

### 5.4 与 Workspace 集成

在 `src/workspace/mod.rs:93-116` 的 `Workspace::open()` 中，索引步骤为可选的后台任务：

```rust
// workspace/mod.rs (概念修改)
impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        // ... 现有逻辑 ...

        // 新增: 初始化认知引擎
        let cognition = CognitionEngineImpl::new(
            &root,
            cognition_config,
            embedding_model,
        )?;

        // 后台增量索引（非阻塞）
        if let Err(e) = cognition.index_project(&root, IndexMode::Incremental { changed_files }) {
            tracing::warn!("Background indexing failed: {}", e);
        }

        Ok(Self { root, config, snapshot_mgr, cognition })
    }
}
```

---

## 6. 跨会话记忆设计

### 6.1 记忆层次

```
┌─────────────────────────────────────────────────────┐
│                  记忆层次架构                          │
├─────────────────────────────────────────────────────┤
│                                                      │
│  L1: 工作记忆 (Working Memory)                       │
│  ├── 当前会话上下文                                   │
│  ├── 最近文件、工具调用历史                            │
│  └── 来源: 现有 WorkingMemory (内存)                  │
│                                                      │
│  L2: 会话摘要 (Session Memory)                       │
│  ├── 会话摘要 + 关键决策                              │
│  ├── 学习到的项目模式                                  │
│  └── 来源: SQLite 持久化 (新增表)                     │
│                                                      │
│  L3: 项目知识 (Project Knowledge)                    │
│  ├── 架构决策、命名约定、依赖关系                      │
│  ├── 知识图谱: 模块关系、概念映射                      │
│  └── 来源: 现有 ProjectMemory + 知识图谱 (增强)       │
│                                                      │
│  L4: 代码索引 (Code Index)                           │
│  ├── 全项目代码块的向量嵌入                            │
│  ├── 文件结构元数据                                   │
│  └── 来源: SQLite + fastembed (新增表)                │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### 6.2 会话摘要生成

会话结束时（或定期），由 LLM 生成结构化摘要：

```rust
/// 会话摘要器
pub struct SessionSummarizer;

impl SessionSummarizer {
    /// 从 WorkingMemory + 对话历史生成结构化摘要
    pub async fn summarize(
        working: &WorkingMemory,
        conversation: &[Message],
        llm: &dyn LlmProvider,
    ) -> Result<SessionMemory> {
        // 1. 提取关键对话片段（最近 10 条工具调用 + 结果）
        // 2. LLM 生成摘要（含决策、模式、上下文）
        // 3. 返回 SessionMemory 结构
    }
}
```

**持久化时机**:
- 每完成一个任务（`WorkingMemory.current_task` 被清除时）
- 用户显式 `zcode session save`
- 会话超时（TUI 闲置 5 分钟）
- 进程退出信号处理

### 6.3 知识图谱

轻量级概念关系图，存储在 SQLite：

```sql
-- 知识图谱节点
CREATE TABLE knowledge_nodes (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL,        -- 'concept', 'module', 'function', 'file'
    name        TEXT NOT NULL,
    description TEXT,
    embedding   BLOB,                -- 向量嵌入 (可选)
    UNIQUE(kind, name)
);

-- 知识图谱边
CREATE TABLE knowledge_edges (
    source_id   INTEGER NOT NULL REFERENCES knowledge_nodes(id),
    target_id   INTEGER NOT NULL REFERENCES knowledge_nodes(id),
    relation    TEXT NOT NULL,        -- 'depends_on', 'contains', 'implements', 'references'
    weight      REAL DEFAULT 1.0,
    PRIMARY KEY(source_id, target_id, relation)
);
```

**图谱构建**: 从代码索引中提取关系：
- `module` contains `function` / `struct`
- `function` references `function`（通过 AST 引用分析）
- `module` depends_on `module`（通过 `use` / `import` 语句）

---

## 7. 知识获取管道

### 7.1 需求分析器 (RequirementAnalyzer)

```rust
/// 需求分析器 — 解析用户需求，规划知识收集
pub struct RequirementAnalyzer;

impl RequirementAnalyzer {
    /// 分析需求，生成知识获取计划
    pub fn analyze(requirement: &str, project_context: &ProjectContext) -> KnowledgePlan {
        // 1. 提取关键技术概念
        // 2. 确定需要的知识类型
        // 3. 按优先级排序知识源
        // 4. 生成执行计划
    }
}

/// 知识获取计划
#[derive(Debug, Clone)]
pub struct KnowledgePlan {
    /// 原始需求
    pub requirement: String,
    /// 提取的概念
    pub concepts: Vec<String>,
    /// 有序的知识源查询列表
    pub steps: Vec<KnowledgeStep>,
}

/// 单个知识获取步骤
#[derive(Debug, Clone)]
pub struct KnowledgeStep {
    /// 知识源类型
    pub source: KnowledgeSourceType,
    /// 查询内容
    pub query: String,
    /// 期望的知识类型
    pub expected_kind: KnowledgeKind,
    /// 优先级 (越高越先执行)
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeSourceType {
    /// 项目代码索引
    CodeIndex,
    /// 项目记忆 (architecture decisions, patterns)
    ProjectMemory,
    /// 会话记忆 (历史决策)
    SessionMemory,
    /// Web 文档
    WebDocumentation,
    /// MCP 工具能力
    McpCapabilities,
    /// 依赖分析
    DependencyAnalysis,
}

/// 项目上下文快照
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub dependencies: Vec<String>,
    pub root: PathBuf,
}
```

### 7.2 管道执行流程

```
用户需求: "实现一个 Redis 缓存层"
    │
    ▼
RequirementAnalyzer::analyze()
    │
    ├── 概念提取: ["Redis", "cache", "layer"]
    │
    ├── 知识计划生成:
    │   Step 1 (p=9): CodeIndex — "Redis" / "cache" (项目是否已有相关代码?)
    │   Step 2 (p=8): ProjectMemory — "architecture" (架构决策中是否有缓存策略?)
    │   Step 3 (p=7): DependencyAnalysis — Cargo.toml 中是否有 redis crate?
    │   Step 4 (p=6): WebDocumentation — redis crate 文档
    │   Step 5 (p=5): SessionMemory — 历史决策中是否讨论过缓存
    │
    ▼
KnowledgeExecutor::execute(plan)
    │
    ├── 并行执行独立步骤
    ├── 收集 KnowledgeFragment
    ├── 去重 + 相关性排序
    │
    ▼
KnowledgeContext (组装完成)
    ├── 项目代码: 0 个相关块
    ├── 项目记忆: "use Redis for session cache" 决策记录
    ├── 依赖分析: Cargo.toml 无 redis 依赖
    ├── Web 文档: redis crate 基本用法
    └── Token 预算内截断
```

### 7.3 KnowledgeContext — LLM 消费的最终产物

```rust
/// 组装完成的知识上下文
#[derive(Debug, Clone)]
pub struct KnowledgeContext {
    /// 各知识源的片段
    pub fragments: Vec<KnowledgeFragment>,
    /// 总 token 估算
    pub estimated_tokens: usize,
    /// 是否有内容因预算截断
    pub truncated: bool,
}

impl KnowledgeContext {
    /// 渲染为可注入 LLM 系统提示的文本
    pub fn render(&self) -> String {
        // 按知识源类型分组渲染
        // 格式: "## Project Code\n...\n## Documentation\n..."
    }
}
```

---

## 8. 外部知识源集成

### 8.1 Web 文档获取

```rust
/// Web 文档知识源
pub struct WebDocSource {
    /// HTTP 客户端 (复用现有 reqwest)
    client: reqwest::Client,
}

impl KnowledgeSource for WebDocSource {
    fn name(&self) -> &str { "web_documentation" }

    fn query(&self, query: &KnowledgeQuery) -> Result<Vec<KnowledgeFragment>> {
        // 1. 基于 concepts 构建搜索 URL
        //    优先: docs.rs/{crate_name}, 官方文档站
        // 2. 获取页面内容
        // 3. 提取纯文本 (去除 HTML 标签)
        // 4. 分块 + 返回片段
    }

    fn relevance(&self, query: &KnowledgeQuery) -> f32 {
        match query.kind {
            KnowledgeKind::DependencyDoc => 0.9,
            KnowledgeKind::ApiUsage => 0.7,
            _ => 0.3,
        }
    }
}
```

**文档源优先级**:
1. `docs.rs` — Rust crate 文档
2. `readthedocs.io` — 通用项目文档
3. MDN / 官方文档站 — 通过已知 URL 模式
4. GitHub README — 依赖库的 README

### 8.2 MCP 能力发现

```rust
/// MCP 能力知识源
pub struct McpCapabilitySource {
    /// 已连接的 MCP 客户端列表
    clients: Vec<Arc<McpClient>>,
}

impl KnowledgeSource for McpCapabilitySource {
    fn name(&self) -> &str { "mcp_capabilities" }

    fn query(&self, query: &KnowledgeQuery) -> Result<Vec<KnowledgeFragment>> {
        // 1. 遍历所有已连接 MCP 客户端
        // 2. 调用 tools/list 获取工具列表
        // 3. 匹配工具名称/描述与查询概念
        // 4. 返回匹配的工具描述作为知识片段
    }

    fn relevance(&self, query: &KnowledgeQuery) -> f32 {
        // 如果需求中涉及 "tool" / "capability" / MCP 相关概念
        0.8
    }
}
```

与现有 `src/mcp/client.rs:70-74` 的 `McpClient` 集成，复用其 `list_tools()` 方法（`mcp/client.rs:140-142`）。

### 8.3 依赖分析

```rust
/// 依赖分析知识源
pub struct DependencySource {
    project_root: PathBuf,
}

impl KnowledgeSource for DependencySource {
    fn name(&self) -> &str { "dependency_analysis" }

    fn query(&self, query: &KnowledgeQuery) -> Result<Vec<KnowledgeFragment>> {
        // 1. 解析 Cargo.toml / package.json / requirements.txt
        // 2. 识别与查询概念相关的依赖
        // 3. 读取 lock 文件获取精确版本
        // 4. 返回依赖描述作为知识片段
    }
}
```

---

## 9. 与现有模块集成方案

### 9.1 替换 SemanticIndex

**现状**: `ContextAssembler::assemble()` 接受 `Option<&mut SemanticIndex>` 参数（`context.rs:156-162`）。

**方案**: 新增 `CognitionEngine` 参数，`SemanticIndex` 保留作为零依赖 fallback。

```rust
// context.rs 修改方案 (概念)
impl ContextAssembler {
    pub fn assemble(
        &self,
        query: &str,
        working: &WorkingMemory,
        project: Option<&ProjectMemory>,
        semantic: Option<&mut SemanticIndex>,      // 保留, fallback
        cognition: Option<&dyn CognitionEngine>,   // 新增, 优先使用
    ) -> AssembledContext {
        // 优先使用 cognition 的高级搜索
        let semantic_results = if let Some(eng) = cognition {
            eng.search(query, 5)
                .into_iter()
                .map(|r| format!("// {} (score: {:.2})\n{}", r.id, r.score, r.text))
                .collect()
        } else if let Some(idx) = semantic {
            idx.search(query, 5)
                .into_iter()
                .map(|r| format!("// {} (score: {:.2})\n{}", r.id, r.score, r.text))
                .collect()
        } else {
            vec![]
        };
        // ... 其余逻辑不变
    }
}
```

### 9.2 增强 Workspace

在 `src/workspace/mod.rs:86-91` 的 `Workspace` struct 中新增 `cognition` 字段：

```rust
pub struct Workspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    snapshot_mgr: Option<SnapshotManager>,
    cognition: Option<CognitionEngineImpl>,  // 新增
}
```

### 9.3 增强 ProjectConfig

在 `src/config/mod.rs:14-59` 的 `ProjectConfig` 中新增认知配置：

```rust
// config/mod.rs 新增
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CognitionConfig {
    /// 启用/禁用认知引擎
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 嵌入模型选择
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// 最大块大小（字符数）
    #[serde(default = "default_max_chunk_size")]
    pub max_chunk_size: usize,

    /// 块重叠大小（字符数）
    #[serde(default = "default_overlap_size")]
    pub overlap_size: usize,

    /// 是否在打开项目时自动索引
    #[serde(default = "default_true")]
    pub auto_index: bool,

    /// 索引排除的路径模式
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// 认知数据库路径
    #[serde(default = "default_cognition_db_path")]
    pub db_path: String,
}

fn default_embedding_model() -> String { "AllMiniLML6V2".to_string() }
fn default_max_chunk_size() -> usize { 1500 }
fn default_overlap_size() -> usize { 100 }
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "target/**".to_string(),
        "node_modules/**".to_string(),
        ".git/**".to_string(),
        "vendor/**".to_string(),
    ]
}
fn default_cognition_db_path() -> String { ".zcode/cognition.db".to_string() }
```

在 `ProjectConfig` 中新增字段：

```rust
pub struct ProjectConfig {
    // ... 现有字段 ...

    /// 认知引擎配置
    #[serde(default)]
    pub cognition: CognitionConfig,
}
```

### 9.4 增强 ContextAssembler

`ContextAssembler::assemble()` 方法（`context.rs:156-255`）的增强策略：

1. **优先使用 CognitionEngine**: 如果可用，替换 `semantic.search()` 为 `cognition.search()`
2. **注入会话记忆**: 在 system prompt 中加入相关历史决策和模式
3. **注入外部知识**: 加入 `KnowledgeContext` 的渲染结果

### 9.5 与 Agent 系统集成

在 `src/agent/loop_exec.rs` 的 AgentLoop 中，当收到用户消息时：

```
User Message
    │
    ▼
RequirementAnalyzer::analyze(message)
    │
    ▼
KnowledgePlan execution (并行)
    │
    ▼
KnowledgeContext assembled
    │
    ▼
注入 ContextAssembler → LLM Prompt
```

---

## 10. 存储设计

### 10.1 SQLite Schema 扩展

使用现有 `rusqlite` (bundled) 依赖（`Cargo.toml:58`），在 `.zcode/cognition.db` 中创建新表：

```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;

-- ═══ 代码索引 ═══

-- 代码块表 (替代 project.rs 中的 code_chunks 表)
CREATE TABLE IF NOT EXISTS code_blocks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    path            TEXT    NOT NULL,
    start_line      INTEGER NOT NULL,
    end_line        INTEGER NOT NULL,
    identifier      TEXT,               -- 函数名/类名
    kind            TEXT    NOT NULL,    -- 'function', 'struct', 'trait', etc.
    content         TEXT    NOT NULL,
    content_hash    TEXT    NOT NULL,    -- blake3 hash of content
    embedding       BLOB,               -- f32 向量, 二进制格式
    model_id        TEXT,               -- 使用的嵌入模型标识
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE INDEX IF NOT EXISTS idx_blocks_path ON code_blocks(path);
CREATE INDEX IF NOT EXISTS idx_blocks_kind ON code_blocks(kind);
CREATE INDEX IF NOT EXISTS idx_blocks_identifier ON code_blocks(identifier);

-- 文件哈希表 (用于增量索引)
CREATE TABLE IF NOT EXISTS file_hashes (
    path            TEXT PRIMARY KEY,
    hash            TEXT    NOT NULL,    -- blake3
    last_indexed    INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- ═══ 会话记忆 ═══

-- 会话摘要表
CREATE TABLE IF NOT EXISTS session_memories (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT    NOT NULL UNIQUE,
    summary         TEXT    NOT NULL,
    task            TEXT,
    decisions_json  TEXT,               -- JSON array of DecisionRecord
    patterns_json   TEXT,               -- JSON array of LearnedPattern
    embedding       BLOB,               -- 摘要的嵌入向量
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_created ON session_memories(created_at);

-- ═══ 知识图谱 ═══

CREATE TABLE IF NOT EXISTS knowledge_nodes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    kind            TEXT    NOT NULL,
    name            TEXT    NOT NULL,
    description     TEXT,
    embedding       BLOB,
    UNIQUE(kind, name)
);

CREATE TABLE IF NOT EXISTS knowledge_edges (
    source_id       INTEGER NOT NULL REFERENCES knowledge_nodes(id),
    target_id       INTEGER NOT NULL REFERENCES knowledge_nodes(id),
    relation        TEXT    NOT NULL,
    weight          REAL    DEFAULT 1.0,
    PRIMARY KEY(source_id, target_id, relation)
);

CREATE INDEX IF NOT EXISTS idx_edges_source ON knowledge_edges(source_id);
CREATE INDEX IF NOT EXISTS idx_edges_target ON knowledge_edges(target_id);

-- ═══ 外部知识缓存 ═══

CREATE TABLE IF NOT EXISTS external_knowledge (
    url             TEXT PRIMARY KEY,
    content         TEXT    NOT NULL,
    content_hash    TEXT    NOT NULL,
    fetched_at      INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    expires_at      INTEGER              -- 缓存过期时间
);

-- ═══ 项目元数据 ═══

CREATE TABLE IF NOT EXISTS project_meta (
    key             TEXT PRIMARY KEY,
    value           TEXT    NOT NULL,
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
```

### 10.2 向量存储与检索

由于 `sqlite-vec` / `sqlite-vss` 需要 MSRV 1.86+（超出我们的 1.75+ 要求），采用**应用层向量搜索**策略：

```rust
/// 向量索引 — 应用层暴力搜索 + 可选 HNSW 优化
pub struct VectorIndex {
    /// 所有向量块
    entries: Vec<VectorEntry>,
    /// 嵌入模型
    model: Box<dyn EmbeddingModel>,
}

struct VectorEntry {
    id: i64,
    path: String,
    start_line: usize,
    end_line: usize,
    identifier: Option<String>,
    vector: Vec<f32>,
}

impl VectorIndex {
    /// 搜索最相似的 k 个向量
    pub fn search(&self, query_vector: &[f32], top_k: usize) -> Vec<SearchResult> {
        // 1. 计算余弦相似度
        let mut scored: Vec<(f32, &VectorEntry)> = self.entries.iter()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                (score, entry)
            })
            .collect();

        // 2. 按 score 降序排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

        // 3. 取 top_k
        scored.into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.5)  // 相似度阈值
            .map(|(score, entry)| SearchResult {
                id: format!("{}:{}-{}", entry.path, entry.start_line, entry.end_line),
                score,
                // 从数据库加载 content
            })
            .collect()
    }
}
```

**性能分析**:
- 10,000 个代码块 × 384 维 = ~15MB 内存
- 暴力搜索 10K 向量 ~5ms（现代 CPU）
- 对于大多数项目（< 50K 块）足够快
- 如果需要更大规模，后续可引入 `hnsw` crate 纯 Rust 实现

### 10.3 向量存储格式

嵌入向量在 SQLite 中以 BLOB 格式存储（比 JSON 节省 4x 空间）：

```rust
/// f32 向量 → BLOB
fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &f in vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// BLOB → f32 向量
fn blob_to_vec(blob: &[u8], dim: usize) -> Vec<f32> {
    blob.chunks_exact(4)
        .take(dim)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
```

384 维向量 = 1,536 bytes BLOB（vs JSON 数组 ~3,500 bytes）。

---

## 11. 配置方案

### 11.1 项目级配置 (`.zcode/config.toml`)

```toml
[cognition]
enabled = true
embedding_model = "AllMiniLML6V2"
max_chunk_size = 1500
overlap_size = 100
auto_index = true
db_path = ".zcode/cognition.db"

[cognition.exclude]
patterns = [
    "target/**",
    "node_modules/**",
    ".git/**",
    "vendor/**",
    "*.lock",
    "*.min.js",
]
```

### 11.2 用户级配置 (`~/.config/zcode/settings.toml`)

```toml
[cognition]
# 嵌入模型缓存目录
model_cache_dir = "~/.cache/zcode/models"
# 全局禁用（所有项目）
disabled = false
# 后台索引线程数
index_threads = 4
```

---

## 12. 文件组织

```
src/
├── cognition/                          # 新增模块
│   ├── mod.rs                          # 公共 API + re-exports
│   ├── engine.rs                       # CognitionEngineImpl 主实现
│   ├── embedding.rs                    # EmbeddingModel trait + fastembed 实现
│   ├── chunker.rs                      # CodeChunker — AST 感知分块
│   ├── vector.rs                       # VectorIndex — 内存向量搜索
│   ├── memory.rs                       # SessionMemory + 跨会话持久化
│   ├── knowledge.rs                    # KnowledgeSource trait + 管道
│   ├── requirement.rs                  # RequirementAnalyzer
│   ├── pipeline.rs                     # KnowledgePlan 执行器
│   ├── sources/                        # 知识源实现
│   │   ├── mod.rs
│   │   ├── web_doc.rs                  # WebDocSource
│   │   ├── mcp_source.rs              # McpCapabilitySource
│   │   ├── dependency.rs              # DependencySource
│   │   └── code_index.rs             # CodeIndexSource (内部代码搜索)
│   ├── storage.rs                      # SQLite schema + 持久化逻辑
│   └── graph.rs                        # KnowledgeGraph 操作
│
├── memory/                             # 现有模块（保留，增强）
│   ├── mod.rs                          # 新增 cognition pub mod
│   ├── working.rs                      # 不变
│   ├── project.rs                      # 不变（认知引擎独立存储）
│   ├── semantic.rs                     # 保留作为 fallback
│   └── context.rs                      # 增强: 接受 cognition 参数
│
├── workspace/
│   └── mod.rs                          # 增强: 持有 cognition 实例
│
├── config/
│   └── mod.rs                          # 增强: CognitionConfig
│
└── lib.rs                              # 新增 pub mod cognition
```

---

## 13. 实现路线图

### Phase 1: 基础嵌入 + 代码索引 (2-3 周)

**目标**: 替换 TF-IDF，实现真正的语义代码搜索

| 任务 | 优先级 | 工作量 | 依赖 |
|------|--------|--------|------|
| T1.1: 添加 `fastembed` 依赖，实现 `EmbeddingModel` trait | P0 | 1d | 无 |
| T1.2: 实现 `CodeChunker`（AST 感知分块） | P0 | 2d | tree-sitter |
| T1.3: 实现 `VectorIndex`（内存向量搜索） | P0 | 1d | T1.1 |
| T1.4: 实现 `storage.rs`（SQLite schema + 读写） | P0 | 1d | rusqlite |
| T1.5: 实现 `CognitionEngineImpl::index_project()` | P0 | 2d | T1.2-T1.4 |
| T1.6: 实现 `CognitionEngineImpl::search()` | P0 | 1d | T1.5 |
| T1.7: 集成到 `ContextAssembler`（替换 SemanticIndex） | P0 | 1d | T1.6 |
| T1.8: 单元测试 + 集成测试 | P0 | 2d | T1.1-T1.7 |

**里程碑**: `ContextAssembler` 默认使用向量搜索，`SemanticIndex` 降级为 fallback。

### Phase 2: 跨会话记忆 (1-2 周)

**目标**: 会话上下文持久化，跨重启保留

| 任务 | 优先级 | 工作量 | 依赖 |
|------|--------|--------|------|
| T2.1: 实现 `SessionMemory` 持久化（SQLite 表） | P1 | 1d | Phase 1 storage |
| T2.2: 实现 `SessionSummarizer`（LLM 生成摘要） | P1 | 1d | LLM provider |
| T2.3: 实现会话记忆召回（向量相似度搜索） | P1 | 1d | T1.3 |
| T2.4: 集成到 `ContextAssembler`（注入历史决策） | P1 | 1d | T2.3 |
| T2.5: 实现增量索引（file_hashes 表 + 变更检测） | P1 | 1d | Phase 1 |
| T2.6: 集成到 `Workspace::open()`（后台索引） | P1 | 1d | T2.5 |
| T2.7: 测试 | P1 | 1d | T2.1-T2.6 |

**里程碑**: 项目打开时自动增量索引 + 上下次会话恢复历史。

### Phase 3: 知识获取管道 (2 周)

**目标**: 主动知识收集，外部知识源集成

| 任务 | 优先级 | 工作量 | 依赖 |
|------|--------|--------|------|
| T3.1: 实现 `RequirementAnalyzer`（概念提取） | P2 | 2d | 无 |
| T3.2: 实现 `KnowledgePlan` + `KnowledgeExecutor` | P2 | 2d | T3.1 |
| T3.3: 实现 `WebDocSource`（docs.rs 抓取） | P2 | 2d | reqwest |
| T3.4: 实现 `McpCapabilitySource` | P3 | 1d | mcp/client.rs |
| T3.5: 实现 `DependencySource` | P3 | 1d | 无 |
| T3.6: 实现 `CodeIndexSource`（内部搜索封装） | P2 | 1d | Phase 1 |
| T3.7: 集成到 AgentLoop（自动知识收集） | P2 | 2d | T3.1-T3.6 |
| T3.8: 测试 | P2 | 1d | T3.1-T3.7 |

**里程碑**: 用户提需求时，系统自动规划并收集相关知识。

### Phase 4: 知识图谱 + 优化 (1-2 周)

**目标**: 概念关系建模，性能优化

| 任务 | 优先级 | 工作量 | 依赖 |
|------|--------|--------|------|
| T4.1: 实现知识图谱构建（从代码索引提取关系） | P3 | 2d | Phase 1 |
| T4.2: 实现图谱查询（概念关联搜索） | P3 | 1d | T4.1 |
| T4.3: 实现外部知识缓存（SQLite + 过期策略） | P3 | 1d | Phase 3 |
| T4.4: 性能优化（并行嵌入、HNSW 评估） | P3 | 2d | Phase 1 |
| T4.5: 配置完善 + 文档 | P3 | 1d | 全部 |

**里程碑**: 完整认知引擎上线。

---

## 附录 A: 错误处理扩展

在 `src/error.rs:9-61` 的 `ZcodeError` 中新增变体：

```rust
#[derive(Error, Debug)]
pub enum ZcodeError {
    // ... 现有变体 ...

    /// 嵌入模型错误
    #[error("Embedding model error: {0}")]
    EmbeddingError(String),

    /// 索引错误
    #[error("Index error: {0}")]
    IndexError(String),

    /// 知识获取错误
    #[error("Knowledge acquisition error: {source}: {message}")]
    KnowledgeError { source: String, message: String },
}
```

## 附录 B: 性能预算

| 操作 | 目标 | 备注 |
|------|------|------|
| 首次全量索引 (10K 文件) | < 60s | 含嵌入计算，batch=32 |
| 增量索引 (100 变更文件) | < 5s | 仅重新嵌入变更文件 |
| 语义搜索 (top-10) | < 10ms | 内存暴力搜索 10K 向量 |
| 单条文本嵌入 | < 5ms | AllMiniLML6V2 on CPU |
| 会话记忆存储 | < 100ms | SQLite INSERT |
| 会话记忆召回 | < 20ms | 向量搜索 + SQLite JOIN |
| Web 文档获取 | < 5s | 含网络请求 |

## 附录 C: 与现有代码的冲突点

| 文件 | 行号 | 冲突类型 | 解决方案 |
|------|------|----------|----------|
| `context.rs` | 156-162 | `assemble()` 签名变更 | 新增 `cognition` 参数（Option，向后兼容） |
| `workspace/mod.rs` | 86-91 | struct 字段变更 | 新增 `cognition: Option<...>` |
| `config/mod.rs` | 14-59 | 新增配置节 | 新增 `cognition: CognitionConfig` 字段（Default impl） |
| `error.rs` | 9-61 | 新增错误变体 | 添加 `EmbeddingError`、`IndexError`、`KnowledgeError` |
| `lib.rs` | 29-46 | 新增模块声明 | 添加 `pub mod cognition` + re-exports |
| `Cargo.toml` | 10-72 | 新增依赖 | 添加 `fastembed = "4"` |

所有变更均为**增量式**：新字段使用 `Option` + `Default`，不破坏现有 API 调用。
