//! TaskGraph — DAG-based task dependency management
//!
//! Supports topological sort, execution levels for parallel scheduling,
//! cycle detection, and critical path analysis.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::agent::Task;

/// Unique task identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TaskId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TaskId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Task execution status within the graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeStatus {
    /// Waiting for dependencies to complete
    Pending,
    /// Dependencies met, ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Executed, awaiting verification
    AwaitingVerification,
    /// Successfully completed
    Completed,
    /// Failed with reason
    Failed { reason: String },
    /// Skipped due to dependency failure
    Skipped,
}

impl std::fmt::Display for TaskNodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::AwaitingVerification => write!(f, "awaiting_verification"),
            Self::Completed => write!(f, "completed"),
            Self::Failed { reason } => write!(f, "failed: {}", reason),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// A node in the task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// The underlying task
    pub task: Task,
    /// Current status
    pub status: TaskNodeStatus,
    /// Snapshot ID before execution
    pub pre_snapshot_id: Option<i64>,
    /// Verification score history
    pub verification_scores: Vec<f64>,
}

impl TaskNode {
    /// Create a new task node from a task
    pub fn new(task: Task) -> Self {
        Self {
            task,
            status: TaskNodeStatus::Pending,
            pre_snapshot_id: None,
            verification_scores: Vec::new(),
        }
    }
}

/// Task dependency graph — a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    /// All task nodes indexed by ID
    nodes: HashMap<TaskId, TaskNode>,
    /// Adjacency: task_id → tasks that depend on it
    dependents: HashMap<TaskId, Vec<TaskId>>,
    /// Adjacency: task_id → tasks it depends on
    dependencies: HashMap<TaskId, Vec<TaskId>>,
}

impl TaskGraph {
    /// Create an empty task graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            dependents: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Build a TaskGraph from a list of tasks and dependency pairs
    ///
    /// `dependencies` is a list of (task_id, depends_on_task_id) pairs
    pub fn build(
        tasks: Vec<Task>,
        deps: Vec<(TaskId, TaskId)>,
    ) -> Result<Self, GraphError> {
        let mut graph = Self::new();

        for task in tasks {
            let id = TaskId(task.id.clone());
            graph.nodes.insert(id, TaskNode::new(task));
        }

        for (task_id, dep_id) in deps {
            // Validate both nodes exist
            if !graph.nodes.contains_key(&task_id) {
                return Err(GraphError::NodeNotFound(task_id.0));
            }
            if !graph.nodes.contains_key(&dep_id) {
                return Err(GraphError::NodeNotFound(dep_id.0));
            }

            graph.dependencies.entry(task_id.clone()).or_default().push(dep_id.clone());
            graph.dependents.entry(dep_id).or_default().push(task_id);
        }

        // Validate no cycles
        graph.validate_no_cycles()?;

        // Mark initial ready tasks
        graph.update_ready_status();

        Ok(graph)
    }

    /// Number of tasks in the graph
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a task node by ID
    pub fn get(&self, id: &TaskId) -> Option<&TaskNode> {
        self.nodes.get(id)
    }

    /// Get a mutable task node by ID
    pub fn get_mut(&mut self, id: &TaskId) -> Option<&mut TaskNode> {
        self.nodes.get_mut(id)
    }

    /// Get all task IDs
    pub fn task_ids(&self) -> Vec<TaskId> {
        self.nodes.keys().cloned().collect()
    }

    /// Get dependencies for a task (tasks it depends on)
    pub fn dependencies_for(&self, id: &TaskId) -> Option<&Vec<TaskId>> {
        self.dependencies.get(id)
    }

    /// Get dependents for a task (tasks that depend on it)
    pub fn dependents_for(&self, id: &TaskId) -> Option<&Vec<TaskId>> {
        self.dependents.get(id)
    }

    /// Check for cycles using DFS
    pub fn validate_no_cycles(&self) -> Result<(), GraphError> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for id in self.nodes.keys() {
            if !visited.contains(id) {
                self.dfs_cycle_check(id, &mut visited, &mut in_stack)?;
            }
        }

        Ok(())
    }

    fn dfs_cycle_check(
        &self,
        id: &TaskId,
        visited: &mut HashSet<TaskId>,
        in_stack: &mut HashSet<TaskId>,
    ) -> Result<(), GraphError> {
        visited.insert(id.clone());
        in_stack.insert(id.clone());

        if let Some(deps) = self.dependencies.get(id) {
            for dep in deps {
                if in_stack.contains(dep) {
                    return Err(GraphError::CircularDependency {
                        from: id.0.clone(),
                        to: dep.0.clone(),
                    });
                }
                if !visited.contains(dep) {
                    self.dfs_cycle_check(dep, visited, in_stack)?;
                }
            }
        }

        in_stack.remove(id);
        Ok(())
    }

    /// Compute execution levels (topological sort layers)
    ///
    /// Tasks in the same level can be executed in parallel.
    pub fn execution_levels(&self) -> Vec<Vec<TaskId>> {
        let mut in_degree: HashMap<&TaskId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }
        for deps in self.dependencies.values() {
            for dep in deps {
                *in_degree.entry(dep).or_insert(0) += 0; // dep is depended ON
            }
            // in_degree for task_id = number of its dependencies
        }

        // Recalculate: in_degree[node] = number of nodes it depends on
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            let count = self.dependencies.get(id).map(|d| d.len()).unwrap_or(0);
            in_degree.insert(id.clone(), count);
        }

        let mut levels = Vec::new();
        let mut completed: HashSet<TaskId> = HashSet::new();

        loop {
            // Find all nodes with in_degree == 0 that aren't completed
            let ready: Vec<TaskId> = in_degree
                .iter()
                .filter(|(id, &deg)| deg == 0 && !completed.contains(id))
                .map(|(id, _)| id.clone())
                .collect();

            if ready.is_empty() {
                break;
            }

            levels.push(ready.clone());

            for id in &ready {
                completed.insert(id.clone());
                // Reduce in_degree for dependents
                if let Some(deps) = self.dependents.get(id) {
                    for dep in deps {
                        if let Some(deg) = in_degree.get_mut(dep) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }
        }

        levels
    }

    /// Get currently ready tasks (dependencies met, not started)
    pub fn ready_tasks(&self) -> Vec<TaskId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.status == TaskNodeStatus::Ready)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Mark a task completed and return newly unblocked task IDs
    pub fn complete_task(&mut self, id: &TaskId) -> Vec<TaskId> {
        if let Some(node) = self.nodes.get_mut(id) {
            node.status = TaskNodeStatus::Completed;
        }

        let mut newly_ready = Vec::new();
        if let Some(dependents) = self.dependents.get(id).cloned() {
            for dep_id in &dependents {
                if self.are_dependencies_met(dep_id) {
                    if let Some(node) = self.nodes.get_mut(dep_id) {
                        if node.status == TaskNodeStatus::Pending {
                            node.status = TaskNodeStatus::Ready;
                            newly_ready.push(dep_id.clone());
                        }
                    }
                }
            }
        }

        newly_ready
    }

    /// Mark a task failed, cascade-skip dependents
    pub fn fail_task(&mut self, id: &TaskId, reason: &str) -> Vec<TaskId> {
        if let Some(node) = self.nodes.get_mut(id) {
            node.status = TaskNodeStatus::Failed {
                reason: reason.to_string(),
            };
        }

        let mut skipped = Vec::new();
        let mut queue = VecDeque::new();

        if let Some(deps) = self.dependents.get(id).cloned() {
            for dep in deps {
                queue.push_back(dep);
            }
        }

        while let Some(dep_id) = queue.pop_front() {
            if let Some(node) = self.nodes.get_mut(&dep_id) {
                if matches!(
                    node.status,
                    TaskNodeStatus::Pending | TaskNodeStatus::Ready
                ) {
                    node.status = TaskNodeStatus::Skipped;
                    skipped.push(dep_id.clone());

                    if let Some(next_deps) = self.dependents.get(&dep_id).cloned() {
                        for next in next_deps {
                            queue.push_back(next);
                        }
                    }
                }
            }
        }

        skipped
    }

    /// Check if all dependencies of a task are completed
    fn are_dependencies_met(&self, id: &TaskId) -> bool {
        self.dependencies
            .get(id)
            .map(|deps| {
                deps.iter().all(|dep_id| {
                    self.nodes
                        .get(dep_id)
                        .map(|n| matches!(n.status, TaskNodeStatus::Completed))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(true)
    }

    /// Update pending tasks to ready if dependencies are met
    fn update_ready_status(&mut self) {
        let ids: Vec<TaskId> = self.nodes.keys().cloned().collect();
        for id in &ids {
            if self.are_dependencies_met(id) {
                if let Some(node) = self.nodes.get_mut(id) {
                    if node.status == TaskNodeStatus::Pending {
                        node.status = TaskNodeStatus::Ready;
                    }
                }
            }
        }
    }

    /// Compute the critical path (longest dependency chain)
    pub fn critical_path(&self) -> Vec<TaskId> {
        let mut longest: Vec<TaskId> = Vec::new();
        for id in self.nodes.keys() {
            let path = self.longest_path_to(id);
            if path.len() > longest.len() {
                longest = path;
            }
        }
        longest
    }

    fn longest_path_to(&self, target: &TaskId) -> Vec<TaskId> {
        let deps = self.dependencies.get(target).cloned().unwrap_or_default();
        if deps.is_empty() {
            return vec![target.clone()];
        }

        let mut best = Vec::new();
        for dep in &deps {
            let path = self.longest_path_to(dep);
            if path.len() > best.len() {
                best = path;
            }
        }

        let mut result = best;
        result.push(target.clone());
        result
    }

    /// Maximum parallelism (widest execution level)
    pub fn max_parallelism(&self) -> usize {
        self.execution_levels()
            .iter()
            .map(|level| level.len())
            .max()
            .unwrap_or(0)
    }

    /// Export as DOT format for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph TaskGraph {\n");
        dot.push_str("    rankdir=TB;\n");
        dot.push_str("    node [shape=box];\n\n");

        for (id, node) in &self.nodes {
            let label = &node.task.description;
            let truncated: String = label.chars().take(40).collect();
            let color = match &node.status {
                TaskNodeStatus::Pending => "gray",
                TaskNodeStatus::Ready => "lightblue",
                TaskNodeStatus::Running => "yellow",
                TaskNodeStatus::Completed => "lightgreen",
                TaskNodeStatus::Failed { .. } => "lightcoral",
                TaskNodeStatus::Skipped => "lightgray",
                TaskNodeStatus::AwaitingVerification => "khaki",
            };
            dot.push_str(&format!(
                "    \"{}\" [label=\"{}\", style=filled, fillcolor={}];\n",
                id.0, truncated, color
            ));
        }

        dot.push('\n');

        for (id, deps) in &self.dependencies {
            for dep in deps {
                dot.push_str(&format!("    \"{}\" -> \"{}\";\n", dep.0, id.0));
            }
        }

        dot.push_str("}\n");
        dot
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph construction/validation errors
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Circular dependency detected: {from} -> {to}")]
    CircularDependency { from: String, to: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Task;

    fn make_task(desc: &str) -> Task {
        let mut t = Task::new(desc);
        t.id = desc.replace(' ', "-");
        t
    }

    #[test]
    fn test_empty_graph() {
        let g = TaskGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(g.execution_levels().is_empty());
    }

    #[test]
    fn test_single_task() {
        let tasks = vec![make_task("task A")];
        let g = TaskGraph::build(tasks, vec![]).unwrap();
        assert_eq!(g.len(), 1);
        let levels = g.execution_levels();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 1);
    }

    #[test]
    fn test_linear_chain() {
        let tasks = vec![make_task("A"), make_task("B"), make_task("C")];
        let deps = vec![
            (TaskId("C".into()), TaskId("B".into())),
            (TaskId("B".into()), TaskId("A".into())),
        ];
        let g = TaskGraph::build(tasks, deps).unwrap();
        let levels = g.execution_levels();
        assert_eq!(levels.len(), 3); // A, then B, then C
    }

    #[test]
    fn test_parallel_tasks() {
        let tasks = vec![make_task("A"), make_task("B"), make_task("C")];
        let deps = vec![
            (TaskId("B".into()), TaskId("A".into())),
            (TaskId("C".into()), TaskId("A".into())),
        ];
        let g = TaskGraph::build(tasks, deps).unwrap();
        let levels = g.execution_levels();
        assert_eq!(levels.len(), 2); // [A], [B, C]
        assert_eq!(levels[1].len(), 2);
        assert_eq!(g.max_parallelism(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let tasks = vec![make_task("A"), make_task("B")];
        let deps = vec![
            (TaskId("A".into()), TaskId("B".into())),
            (TaskId("B".into()), TaskId("A".into())),
        ];
        let result = TaskGraph::build(tasks, deps);
        assert!(matches!(result, Err(GraphError::CircularDependency { .. })));
    }

    #[test]
    fn test_node_not_found() {
        let tasks = vec![make_task("A")];
        let deps = vec![(TaskId("A".into()), TaskId("Z".into()))];
        let result = TaskGraph::build(tasks, deps);
        assert!(matches!(result, Err(GraphError::NodeNotFound(_))));
    }

    #[test]
    fn test_complete_task() {
        let tasks = vec![make_task("A"), make_task("B")];
        let deps = vec![(TaskId("B".into()), TaskId("A".into()))];
        let mut g = TaskGraph::build(tasks, deps).unwrap();

        // A is ready, B is pending
        assert_eq!(g.ready_tasks().len(), 1);

        let newly_ready = g.complete_task(&TaskId("A".into()));
        assert_eq!(newly_ready.len(), 1);
        assert_eq!(newly_ready[0], TaskId("B".into()));
        assert_eq!(g.ready_tasks().len(), 1);
    }

    #[test]
    fn test_fail_task_cascades() {
        let tasks = vec![make_task("A"), make_task("B"), make_task("C")];
        let deps = vec![
            (TaskId("B".into()), TaskId("A".into())),
            (TaskId("C".into()), TaskId("B".into())),
        ];
        let mut g = TaskGraph::build(tasks, deps).unwrap();

        let skipped = g.fail_task(&TaskId("A".into()), "error");
        assert_eq!(skipped.len(), 2); // B and C should be skipped
    }

    #[test]
    fn test_critical_path() {
        let tasks = vec![make_task("A"), make_task("B"), make_task("C"), make_task("D")];
        let deps = vec![
            (TaskId("B".into()), TaskId("A".into())),
            (TaskId("C".into()), TaskId("A".into())),
            (TaskId("D".into()), TaskId("C".into())),
        ];
        let g = TaskGraph::build(tasks, deps).unwrap();
        let path = g.critical_path();
        assert!(path.len() >= 3); // A → C → D
    }

    #[test]
    fn test_to_dot() {
        let tasks = vec![make_task("A"), make_task("B")];
        let deps = vec![(TaskId("B".into()), TaskId("A".into()))];
        let g = TaskGraph::build(tasks, deps).unwrap();
        let dot = g.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", TaskNodeStatus::Pending), "pending");
        assert_eq!(format!("{}", TaskNodeStatus::Completed), "completed");
        assert_eq!(
            format!("{}", TaskNodeStatus::Failed { reason: "err".into() }),
            "failed: err"
        );
    }
}
