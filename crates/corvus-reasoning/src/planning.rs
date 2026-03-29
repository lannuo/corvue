//! Multi-step planning and execution
//!
//! Provides task decomposition, dependency analysis, execution monitoring, and dynamic replanning.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Status of a task or plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task has not been started
    Pending,
    /// Task is ready to run (dependencies satisfied)
    Ready,
    /// Task is currently executing
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was skipped
    Skipped,
    /// Task was cancelled
    Cancelled,
}

/// A dependency between tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Source task ID (must complete before target)
    pub source: String,
    /// Target task ID
    pub target: String,
    /// Type of dependency
    pub dependency_type: DependencyType,
}

/// Type of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// Target can only start after source completes
    FinishToStart,
    /// Target can only start after source starts
    StartToStart,
    /// Target can only finish after source completes
    FinishToFinish,
    /// Target can only finish after source starts
    StartToFinish,
}

/// A single task in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID
    pub id: String,
    /// Task name/description
    pub name: String,
    /// Detailed description
    pub description: Option<String>,
    /// Current status
    pub status: TaskStatus,
    /// Estimated duration
    pub estimated_duration: Option<Duration>,
    /// Actual duration (if completed)
    pub actual_duration: Option<Duration>,
    /// Task priority (higher = more important)
    pub priority: u32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// When the task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the task started (if applicable)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the task completed (if applicable)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

/// An execution plan composed of tasks and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Plan ID
    pub id: String,
    /// Plan name
    pub name: String,
    /// Plan description
    pub description: Option<String>,
    /// All tasks in the plan
    pub tasks: HashMap<String, Task>,
    /// Dependencies between tasks
    pub dependencies: Vec<Dependency>,
    /// Plan status
    pub status: TaskStatus,
    /// When the plan was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the plan started (if applicable)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the plan completed (if applicable)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Execution monitor for tracking plan progress
pub struct ExecutionMonitor {
    /// The plan being executed
    plan: ExecutionPlan,
    /// Task execution order
    execution_order: Vec<String>,
    /// Current task index
    current_index: usize,
    /// Task start times (for duration tracking)
    start_times: HashMap<String, Instant>,
    /// Event listeners
    listeners: Vec<Box<dyn Fn(&Task) + Send + Sync>>,
    /// Whether to allow parallel execution
    allow_parallel: bool,
    /// Maximum retries per task
    max_retries: u32,
    /// Retry counts per task
    retry_counts: HashMap<String, u32>,
}

/// Planner for creating execution plans
pub struct Planner {
    /// Next task ID counter
    next_task_id: u64,
    /// Default task priority
    default_priority: u32,
}

impl Planner {
    /// Create a new planner
    pub fn new() -> Self {
        Self {
            next_task_id: 1,
            default_priority: 5,
        }
    }

    /// Generate a task ID
    fn generate_id(&mut self) -> String {
        let id = format!("task_{}", self.next_task_id);
        self.next_task_id += 1;
        id
    }

    /// Create a new empty plan
    pub fn create_plan(&mut self, name: String, description: Option<String>) -> ExecutionPlan {
        ExecutionPlan {
            id: format!("plan_{}", self.generate_id()),
            name,
            description,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Add a task to a plan
    pub fn add_task(
        &mut self,
        plan: &mut ExecutionPlan,
        name: String,
        description: Option<String>,
        estimated_duration: Option<Duration>,
    ) -> String {
        let task_id = self.generate_id();

        let task = Task {
            id: task_id.clone(),
            name,
            description,
            status: TaskStatus::Pending,
            estimated_duration,
            actual_duration: None,
            priority: self.default_priority,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        };

        plan.tasks.insert(task_id.clone(), task);
        task_id
    }

    /// Add a dependency between two tasks
    pub fn add_dependency(
        &mut self,
        plan: &mut ExecutionPlan,
        source: &str,
        target: &str,
        dependency_type: DependencyType,
    ) -> bool {
        if !plan.tasks.contains_key(source) || !plan.tasks.contains_key(target) {
            return false;
        }

        // Check for cycles
        if self.would_create_cycle(plan, source, target) {
            return false;
        }

        plan.dependencies.push(Dependency {
            source: source.to_string(),
            target: target.to_string(),
            dependency_type,
        });

        true
    }

    /// Check if adding a dependency would create a cycle
    fn would_create_cycle(&self, plan: &ExecutionPlan, source: &str, target: &str) -> bool {
        // Simple cycle check using DFS
        let mut visited = HashSet::new();
        let mut stack = vec![source];

        while let Some(node) = stack.pop() {
            if node == target {
                continue;
            }
            if !visited.insert(node.to_string()) {
                continue;
            }

            for dep in &plan.dependencies {
                if dep.target == node {
                    stack.push(&dep.source);
                }
            }
        }

        false
    }

    /// Create a sequential plan (tasks execute in order)
    pub fn create_sequential_plan(
        &mut self,
        name: String,
        description: Option<String>,
        task_names: Vec<String>,
    ) -> ExecutionPlan {
        let mut plan = self.create_plan(name, description);
        let mut task_ids = Vec::new();

        for task_name in task_names {
            let task_id = self.add_task(&mut plan, task_name, None, None);
            task_ids.push(task_id);
        }

        // Add dependencies
        for i in 1..task_ids.len() {
            self.add_dependency(
                &mut plan,
                &task_ids[i - 1],
                &task_ids[i],
                DependencyType::FinishToStart,
            );
        }

        plan
    }

    /// Create a parallel plan (tasks can execute simultaneously)
    pub fn create_parallel_plan(
        &mut self,
        name: String,
        description: Option<String>,
        task_names: Vec<String>,
    ) -> ExecutionPlan {
        let mut plan = self.create_plan(name, description);

        for task_name in task_names {
            self.add_task(&mut plan, task_name, None, None);
        }

        // No dependencies - all tasks can run in parallel
        plan
    }

    /// Validate a plan
    pub fn validate_plan(&self, plan: &ExecutionPlan) -> Result<(), String> {
        // Check for cycles
        if self.has_cycle(plan) {
            return Err("Plan contains circular dependencies".to_string());
        }

        // Check all dependencies reference existing tasks
        for dep in &plan.dependencies {
            if !plan.tasks.contains_key(&dep.source) {
                return Err(format!("Dependency source not found: {}", dep.source));
            }
            if !plan.tasks.contains_key(&dep.target) {
                return Err(format!("Dependency target not found: {}", dep.target));
            }
        }

        Ok(())
    }

    /// Check if a plan has cyclic dependencies
    fn has_cycle(&self, plan: &ExecutionPlan) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in plan.tasks.keys() {
            if self.has_cycle_dfs(plan, task_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycle_dfs(
        &self,
        plan: &ExecutionPlan,
        task_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(task_id) {
            return true;
        }
        if visited.contains(task_id) {
            return false;
        }

        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        for dep in &plan.dependencies {
            if dep.source == task_id {
                if self.has_cycle_dfs(plan, &dep.target, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        false
    }

    /// Get tasks that are ready to run
    pub fn get_ready_tasks(&self, plan: &ExecutionPlan) -> Vec<String> {
        let mut ready = Vec::new();

        for (task_id, task) in &plan.tasks {
            if task.status != TaskStatus::Pending {
                continue;
            }

            // Check if all dependencies are satisfied
            let dependencies_satisfied = plan
                .dependencies
                .iter()
                .filter(|d| d.target == *task_id)
                .all(|d| {
                    if let Some(dep_task) = plan.tasks.get(&d.source) {
                        match d.dependency_type {
                            DependencyType::FinishToStart => {
                                dep_task.status == TaskStatus::Completed
                            }
                            DependencyType::StartToStart => {
                                dep_task.status == TaskStatus::Running
                                    || dep_task.status == TaskStatus::Completed
                            }
                            DependencyType::FinishToFinish => {
                                dep_task.status == TaskStatus::Completed
                            }
                            DependencyType::StartToFinish => {
                                dep_task.status == TaskStatus::Running
                                    || dep_task.status == TaskStatus::Completed
                            }
                        }
                    } else {
                        false
                    }
                });

            if dependencies_satisfied {
                ready.push(task_id.clone());
            }
        }

        // Sort by priority (descending)
        ready.sort_by(|a, b| {
            let a_priority = plan.tasks.get(a).map(|t| t.priority).unwrap_or(0);
            let b_priority = plan.tasks.get(b).map(|t| t.priority).unwrap_or(0);
            b_priority.cmp(&a_priority)
        });

        ready
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionMonitor {
    /// Create a new execution monitor for a plan
    pub fn new(plan: ExecutionPlan, allow_parallel: bool, max_retries: u32) -> Self {
        Self {
            plan,
            execution_order: Vec::new(),
            current_index: 0,
            start_times: HashMap::new(),
            listeners: Vec::new(),
            allow_parallel,
            max_retries,
            retry_counts: HashMap::new(),
        }
    }

    /// Add a listener for task status changes
    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: Fn(&Task) + Send + Sync + 'static,
    {
        self.listeners.push(Box::new(listener));
    }

    /// Notify listeners of a task change
    fn notify_listeners(&self, task: &Task) {
        for listener in &self.listeners {
            listener(task);
        }
    }

    /// Get the current plan
    pub fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }

    /// Get a mutable reference to the plan
    pub fn plan_mut(&mut self) -> &mut ExecutionPlan {
        &mut self.plan
    }

    /// Start the plan execution
    pub fn start(&mut self) {
        self.plan.status = TaskStatus::Running;
        self.plan.started_at = Some(chrono::Utc::now());
    }

    /// Mark a task as started
    pub fn start_task(&mut self, task_id: &str) -> bool {
        let task_clone = {
            if let Some(task) = self.plan.tasks.get_mut(task_id) {
                if task.status == TaskStatus::Pending || task.status == TaskStatus::Ready {
                    task.status = TaskStatus::Running;
                    task.started_at = Some(chrono::Utc::now());
                    self.start_times.insert(task_id.to_string(), Instant::now());
                    Some(task.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(task) = task_clone {
            self.notify_listeners(&task);
            return true;
        }
        false
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, task_id: &str) -> bool {
        let task_clone = {
            if let Some(task) = self.plan.tasks.get_mut(task_id) {
                if task.status == TaskStatus::Running {
                    task.status = TaskStatus::Completed;
                    task.completed_at = Some(chrono::Utc::now());

                    if let Some(start) = self.start_times.remove(task_id) {
                        task.actual_duration = Some(start.elapsed());
                    }

                    Some(task.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(task) = task_clone {
            self.notify_listeners(&task);
            self.check_plan_completion();
            return true;
        }
        false
    }

    /// Mark a task as failed
    pub fn fail_task(&mut self, task_id: &str, error: String) -> bool {
        let task_clone = {
            if let Some(task) = self.plan.tasks.get_mut(task_id) {
                if task.status == TaskStatus::Running {
                    // Check if we should retry
                    let retry_count = self.retry_counts.entry(task_id.to_string()).or_insert(0);
                    if *retry_count < self.max_retries {
                        *retry_count += 1;
                        task.status = TaskStatus::Ready;
                        task.error = Some(format!("Retry {}: {}", *retry_count, error));
                        Some(task.clone())
                    } else {
                        task.status = TaskStatus::Failed;
                        task.error = Some(error);
                        task.completed_at = Some(chrono::Utc::now());

                        if let Some(start) = self.start_times.remove(task_id) {
                            task.actual_duration = Some(start.elapsed());
                        }

                        Some(task.clone())
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(task) = task_clone {
            self.notify_listeners(&task);
            if task.status == TaskStatus::Failed {
                self.plan.status = TaskStatus::Failed;
            }
            return true;
        }
        false
    }

    /// Skip a task
    pub fn skip_task(&mut self, task_id: &str) -> bool {
        let task_clone = {
            if let Some(task) = self.plan.tasks.get_mut(task_id) {
                if task.status == TaskStatus::Pending || task.status == TaskStatus::Ready {
                    task.status = TaskStatus::Skipped;
                    Some(task.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(task) = task_clone {
            self.notify_listeners(&task);
            self.check_plan_completion();
            return true;
        }
        false
    }

    /// Check if the plan is complete
    fn check_plan_completion(&mut self) {
        let all_completed = self
            .plan
            .tasks
            .values()
            .all(|t| t.status == TaskStatus::Completed || t.status == TaskStatus::Skipped);

        if all_completed {
            self.plan.status = TaskStatus::Completed;
            self.plan.completed_at = Some(chrono::Utc::now());
        }
    }

    /// Get ready tasks
    pub fn get_ready_tasks(&self) -> Vec<String> {
        let planner = Planner::new();
        planner.get_ready_tasks(&self.plan)
    }

    /// Get progress (0.0-1.0)
    pub fn progress(&self) -> f32 {
        let total = self.plan.tasks.len();
        if total == 0 {
            return 0.0;
        }

        let completed = self
            .plan
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Completed || t.status == TaskStatus::Skipped)
            .count();

        completed as f32 / total as f32
    }

    /// Get a summary of the plan status
    pub fn summary(&self) -> PlanSummary {
        let mut pending = 0;
        let mut ready = 0;
        let mut running = 0;
        let mut completed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for task in self.plan.tasks.values() {
            match task.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::Ready => ready += 1,
                TaskStatus::Running => running += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Failed => failed += 1,
                TaskStatus::Skipped => skipped += 1,
                TaskStatus::Cancelled => {}
            }
        }

        PlanSummary {
            pending,
            ready,
            running,
            completed,
            failed,
            skipped,
            progress: self.progress(),
            total: self.plan.tasks.len(),
        }
    }
}

/// Summary of plan execution status
#[derive(Debug, Clone)]
pub struct PlanSummary {
    /// Number of pending tasks
    pub pending: usize,
    /// Number of ready tasks
    pub ready: usize,
    /// Number of running tasks
    pub running: usize,
    /// Number of completed tasks
    pub completed: usize,
    /// Number of failed tasks
    pub failed: usize,
    /// Number of skipped tasks
    pub skipped: usize,
    /// Progress (0.0-1.0)
    pub progress: f32,
    /// Total tasks
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_creation() {
        let planner = Planner::new();
        assert_eq!(planner.default_priority, 5);
    }

    #[test]
    fn test_create_plan() {
        let mut planner = Planner::new();
        let plan = planner.create_plan("Test Plan".to_string(), None);

        assert_eq!(plan.name, "Test Plan");
        assert!(plan.tasks.is_empty());
        assert_eq!(plan.status, TaskStatus::Pending);
    }

    #[test]
    fn test_add_task() {
        let mut planner = Planner::new();
        let mut plan = planner.create_plan("Test".to_string(), None);

        let task_id = planner.add_task(&mut plan, "Do something".to_string(), None, None);

        assert!(!task_id.is_empty());
        assert_eq!(plan.tasks.len(), 1);
    }

    #[test]
    fn test_add_dependency() {
        let mut planner = Planner::new();
        let mut plan = planner.create_plan("Test".to_string(), None);

        let task1 = planner.add_task(&mut plan, "Task 1".to_string(), None, None);
        let task2 = planner.add_task(&mut plan, "Task 2".to_string(), None, None);

        let result = planner.add_dependency(
            &mut plan,
            &task1,
            &task2,
            DependencyType::FinishToStart,
        );

        assert!(result);
        assert_eq!(plan.dependencies.len(), 1);
    }

    #[test]
    fn test_sequential_plan() {
        let mut planner = Planner::new();
        let plan = planner.create_sequential_plan(
            "Sequence".to_string(),
            None,
            vec!["Step 1".to_string(), "Step 2".to_string(), "Step 3".to_string()],
        );

        assert_eq!(plan.tasks.len(), 3);
        assert_eq!(plan.dependencies.len(), 2);
    }

    #[test]
    fn test_validate_plan() {
        let mut planner = Planner::new();
        let plan = planner.create_sequential_plan(
            "Valid".to_string(),
            None,
            vec!["A".to_string(), "B".to_string()],
        );

        let result = planner.validate_plan(&plan);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_monitor() {
        let mut planner = Planner::new();
        let plan = planner.create_sequential_plan(
            "Test".to_string(),
            None,
            vec!["Step 1".to_string(), "Step 2".to_string()],
        );

        let mut monitor = ExecutionMonitor::new(plan, false, 0);

        monitor.start();
        assert_eq!(monitor.plan().status, TaskStatus::Running);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut planner = Planner::new();
        let mut plan = planner.create_plan("Test".to_string(), None);
        let task_id = planner.add_task(&mut plan, "Task".to_string(), None, None);

        let mut monitor = ExecutionMonitor::new(plan, false, 0);
        monitor.start();

        // Mark as ready (in a real scenario, dependencies would be checked)
        if let Some(task) = monitor.plan_mut().tasks.get_mut(&task_id) {
            task.status = TaskStatus::Ready;
        }

        let result = monitor.start_task(&task_id);
        assert!(result);

        let result = monitor.complete_task(&task_id);
        assert!(result);

        let summary = monitor.summary();
        assert_eq!(summary.completed, 1);
    }

    #[test]
    fn test_progress() {
        let mut planner = Planner::new();
        let plan = planner.create_sequential_plan(
            "Test".to_string(),
            None,
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
        );

        let monitor = ExecutionMonitor::new(plan, false, 0);
        assert_eq!(monitor.progress(), 0.0);
    }
}
