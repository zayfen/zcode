//! PlanVerifier — validates planner output before execution

use crate::execution::graph::TaskGraph;

/// Plan verification result
#[derive(Debug)]
pub struct PlanVerificationResult {
    /// Whether the plan is valid
    pub valid: bool,
    /// Issues found
    pub issues: Vec<PlanIssue>,
    /// Estimated total tokens
    pub estimated_tokens: u64,
    /// Estimated duration in seconds
    pub estimated_duration_secs: u64,
    /// Number of tasks
    pub task_count: usize,
    /// Maximum parallelism
    pub max_parallelism: usize,
    /// Critical path length
    pub critical_path_length: usize,
}

/// A plan-level issue
#[derive(Debug)]
pub struct PlanIssue {
    pub issue_type: PlanIssueType,
    pub description: String,
}

/// Types of plan issues
#[derive(Debug)]
pub enum PlanIssueType {
    CircularDependency,
    IsolatedTasks,
    DeepChain,
    EmptyPlan,
    TooManyTasks,
}

/// Plan verifier — validates TaskGraph before execution
pub struct PlanVerifier {
    /// Maximum allowed dependency chain depth
    pub max_chain_depth: usize,
    /// Maximum allowed number of tasks
    pub max_tasks: usize,
    /// Estimated tokens per task
    pub tokens_per_task: u64,
    /// Estimated seconds per task
    pub seconds_per_task: u64,
}

impl Default for PlanVerifier {
    fn default() -> Self {
        Self {
            max_chain_depth: 5,
            max_tasks: 50,
            tokens_per_task: 10_000,
            seconds_per_task: 60,
        }
    }
}

impl PlanVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify a task graph
    pub fn verify(&self, graph: &TaskGraph) -> PlanVerificationResult {
        let mut issues = Vec::new();

        // 1. Empty plan
        if graph.is_empty() {
            issues.push(PlanIssue {
                issue_type: PlanIssueType::EmptyPlan,
                description: "Plan is empty — no tasks to execute".into(),
            });
            return PlanVerificationResult {
                valid: false,
                issues,
                estimated_tokens: 0,
                estimated_duration_secs: 0,
                task_count: 0,
                max_parallelism: 0,
                critical_path_length: 0,
            };
        }

        // 2. Too many tasks
        let task_count = graph.len();
        if task_count > self.max_tasks {
            issues.push(PlanIssue {
                issue_type: PlanIssueType::TooManyTasks,
                description: format!(
                    "Plan has {} tasks, exceeding limit of {}",
                    task_count, self.max_tasks
                ),
            });
        }

        // 3. Isolated tasks (no dependencies, no dependents)
        let isolated = self.find_isolated_tasks(graph);
        if !isolated.is_empty() {
            issues.push(PlanIssue {
                issue_type: PlanIssueType::IsolatedTasks,
                description: format!(
                    "{} isolated task(s) with no dependencies: {:?}",
                    isolated.len(),
                    isolated.iter().map(|id| id.to_string()).collect::<Vec<_>>()
                ),
            });
        }

        // 4. Deep dependency chain
        let critical_path = graph.critical_path();
        let critical_path_length = critical_path.len();
        if critical_path_length > self.max_chain_depth {
            issues.push(PlanIssue {
                issue_type: PlanIssueType::DeepChain,
                description: format!(
                    "Dependency chain depth is {}, exceeding limit of {}. Consider parallelizing.",
                    critical_path_length, self.max_chain_depth
                ),
            });
        }

        // 5. Resource estimation
        let estimated_tokens = task_count as u64 * self.tokens_per_task;
        let max_parallelism = graph.max_parallelism();
        let estimated_duration_secs = if max_parallelism > 0 {
            let levels = graph.execution_levels();
            levels.len() as u64 * self.seconds_per_task
        } else {
            task_count as u64 * self.seconds_per_task
        };

        let valid = issues.is_empty()
            || issues.iter().all(|i| !matches!(
                i.issue_type,
                PlanIssueType::CircularDependency | PlanIssueType::EmptyPlan
            ));

        PlanVerificationResult {
            valid,
            issues,
            estimated_tokens,
            estimated_duration_secs,
            task_count,
            max_parallelism,
            critical_path_length,
        }
    }

    fn find_isolated_tasks(&self, graph: &TaskGraph) -> Vec<crate::execution::graph::TaskId> {
        // Tasks with no dependencies and no dependents (and there's more than 1 task)
        if graph.len() <= 1 {
            return vec![];
        }

        let mut isolated = Vec::new();
        for id in graph.task_ids() {
            let has_deps = graph.dependencies_for(&id).is_some_and(|deps| !deps.is_empty());
            let has_dependents = graph.dependents_for(&id).is_some_and(|deps| !deps.is_empty());
            if !has_deps && !has_dependents {
                isolated.push(id);
            }
        }
        isolated
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Task;
    use crate::execution::graph::TaskId;

    fn make_task(desc: &str) -> Task {
        let mut t = Task::new(desc);
        t.id = desc.replace(' ', "-");
        t
    }

    #[test]
    fn test_empty_plan() {
        let graph = TaskGraph::new();
        let result = PlanVerifier::new().verify(&graph);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| matches!(i.issue_type, PlanIssueType::EmptyPlan)));
    }

    #[test]
    fn test_single_task_valid() {
        let tasks = vec![make_task("A")];
        let graph = TaskGraph::build(tasks, vec![]).unwrap();
        let result = PlanVerifier::new().verify(&graph);
        assert!(result.valid);
        assert_eq!(result.task_count, 1);
    }

    #[test]
    fn test_too_many_tasks() {
        let verifier = PlanVerifier {
            max_tasks: 2,
            ..Default::default()
        };
        let tasks = vec![make_task("A"), make_task("B"), make_task("C")];
        let graph = TaskGraph::build(tasks, vec![]).unwrap();
        let result = verifier.verify(&graph);
        assert!(result.issues.iter().any(|i| matches!(i.issue_type, PlanIssueType::TooManyTasks)));
    }

    #[test]
    fn test_resource_estimation() {
        let tasks = vec![make_task("A"), make_task("B"), make_task("C")];
        let deps = vec![
            (TaskId("C".into()), TaskId("B".into())),
            (TaskId("B".into()), TaskId("A".into())),
        ];
        let graph = TaskGraph::build(tasks, deps).unwrap();
        let result = PlanVerifier::new().verify(&graph);
        assert_eq!(result.task_count, 3);
        assert_eq!(result.estimated_tokens, 30_000);
        assert_eq!(result.max_parallelism, 1);
    }
}
