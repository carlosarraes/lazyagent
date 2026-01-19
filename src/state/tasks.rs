use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    #[serde(default)]
    pub depends: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksFile {
    pub tasks: Vec<Task>,
}

impl TasksFile {
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "Failed to read tasks YAML file: {}",
                path.as_ref().display()
            )
        })?;

        let tasks_file: TasksFile = serde_yaml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse tasks YAML file: {}",
                path.as_ref().display()
            )
        })?;

        tasks_file.validate()?;

        Ok(tasks_file)
    }

    pub fn total_tasks(&self) -> usize {
        self.tasks.len()
    }

    pub fn completed_tasks(&self) -> usize {
        self.tasks.iter().filter(|t| t.completed).count()
    }

    pub fn remaining_tasks(&self) -> usize {
        self.tasks.iter().filter(|t| !t.completed).count()
    }

    pub fn incomplete_tasks(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| !t.completed).collect()
    }

    pub fn completed_task_list(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.completed).collect()
    }

    pub fn validate(&self) -> Result<()> {
        self.check_unique_ids()?;
        self.check_dependency_refs()?;
        self.check_no_cycles()?;
        Ok(())
    }

    fn check_unique_ids(&self) -> Result<()> {
        let mut seen = HashSet::new();
        for task in &self.tasks {
            if !seen.insert(&task.id) {
                bail!("Duplicate task ID: {}", task.id);
            }
        }
        Ok(())
    }

    fn check_dependency_refs(&self) -> Result<()> {
        let valid_ids: HashSet<&String> = self.tasks.iter().map(|t| &t.id).collect();
        for task in &self.tasks {
            for dep_id in &task.depends {
                if !valid_ids.contains(dep_id) {
                    bail!(
                        "Task '{}' depends on non-existent task '{}'",
                        task.id,
                        dep_id
                    );
                }
            }
        }
        Ok(())
    }

    fn check_no_cycles(&self) -> Result<()> {
        let mut in_degree: HashMap<&String, usize> = HashMap::new();
        let mut adj_list: HashMap<&String, Vec<&String>> = HashMap::new();

        for task in &self.tasks {
            in_degree.insert(&task.id, task.depends.len());
            adj_list.insert(&task.id, Vec::new());
        }

        for task in &self.tasks {
            for dep_id in &task.depends {
                adj_list.get_mut(dep_id).unwrap().push(&task.id);
            }
        }

        let mut queue: Vec<&String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut processed = 0;

        while let Some(task_id) = queue.pop() {
            processed += 1;
            if let Some(dependents) = adj_list.get(task_id) {
                for dependent_id in dependents {
                    let degree = in_degree.get_mut(dependent_id).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(dependent_id);
                    }
                }
            }
        }

        if processed != self.tasks.len() {
            bail!("Circular dependency detected in tasks");
        }

        Ok(())
    }

    pub fn get_task_by_id(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_ready_tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| {
                !task.completed
                    && task
                        .depends
                        .iter()
                        .all(|dep_id| self.get_task_by_id(dep_id).map_or(false, |t| t.completed))
            })
            .collect()
    }

    pub fn get_blocked_tasks(&self) -> Vec<(&Task, Vec<String>)> {
        self.tasks
            .iter()
            .filter(|task| !task.completed)
            .filter_map(|task| {
                let unsatisfied: Vec<String> = task
                    .depends
                    .iter()
                    .filter(|dep_id| {
                        self.get_task_by_id(dep_id)
                            .map_or(true, |t| !t.completed)
                    })
                    .cloned()
                    .collect();
                if unsatisfied.is_empty() {
                    None
                } else {
                    Some((task, unsatisfied))
                }
            })
            .collect()
    }

    pub fn topological_order(&self) -> Result<Vec<&Task>> {
        let mut in_degree: HashMap<&String, usize> = HashMap::new();
        let mut adj_list: HashMap<&String, Vec<&String>> = HashMap::new();

        for task in &self.tasks {
            in_degree.insert(&task.id, task.depends.len());
            adj_list.insert(&task.id, Vec::new());
        }

        for task in &self.tasks {
            for dep_id in &task.depends {
                adj_list.get_mut(dep_id).unwrap().push(&task.id);
            }
        }

        let mut queue: Vec<&String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::new();

        while let Some(task_id) = queue.pop() {
            if let Some(task) = self.get_task_by_id(task_id) {
                result.push(task);
            }
            if let Some(dependents) = adj_list.get(task_id) {
                for dependent_id in dependents {
                    let degree = in_degree.get_mut(dependent_id).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(dependent_id);
                    }
                }
            }
        }

        if result.len() != self.tasks.len() {
            return Err(anyhow!("Circular dependency detected in tasks"));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_task_deserialization() {
        let yaml = r#"
tasks:
  - id: "task-1"
    title: "Task 1"
    completed: true
    depends: []
  - id: "task-2"
    title: "Task 2"
    completed: false
    depends: ["task-1"]
  - id: "task-3"
    title: "Task 3"
    completed: false
    depends: []
"#;

        let tasks_file: TasksFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tasks_file.tasks.len(), 3);
        assert_eq!(tasks_file.tasks[0].id, "task-1");
        assert_eq!(tasks_file.tasks[0].title, "Task 1");
        assert!(tasks_file.tasks[0].completed);
        assert_eq!(tasks_file.tasks[0].depends, Vec::<String>::new());
        assert_eq!(tasks_file.tasks[1].id, "task-2");
        assert_eq!(tasks_file.tasks[1].title, "Task 2");
        assert!(!tasks_file.tasks[1].completed);
        assert_eq!(tasks_file.tasks[1].depends, vec!["task-1"]);
        assert_eq!(tasks_file.tasks[2].id, "task-3");
        assert_eq!(tasks_file.tasks[2].title, "Task 3");
        assert!(!tasks_file.tasks[2].completed);
        assert_eq!(tasks_file.tasks[2].depends, Vec::<String>::new());
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = std::env::temp_dir();
        let tasks_path = temp_dir.join("test_tasks.yaml");
        let mut file = fs::File::create(&tasks_path).unwrap();
        writeln!(
            file,
            r#"
tasks:
  - id: "init-project"
    title: "Initialize project"
    completed: true
    depends: []
  - id: "setup-config"
    title: "Setup config"
    completed: false
    depends: ["init-project"]
"#
        )
        .unwrap();

        let result = TasksFile::load_from(&tasks_path);
        assert!(result.is_ok());
        let tasks_file = result.unwrap();
        assert_eq!(tasks_file.tasks.len(), 2);
        assert_eq!(tasks_file.tasks[0].id, "init-project");
        assert_eq!(tasks_file.tasks[0].title, "Initialize project");
        assert!(tasks_file.tasks[0].completed);

        fs::remove_file(&tasks_path).unwrap();
    }

    #[test]
    fn test_load_from_missing_file() {
        let result = TasksFile::load_from("/nonexistent/path/tasks.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to read tasks YAML file"));
    }

    #[test]
    fn test_load_from_invalid_yaml() {
        let temp_dir = std::env::temp_dir();
        let tasks_path = temp_dir.join("invalid_tasks.yaml");
        let mut file = fs::File::create(&tasks_path).unwrap();
        writeln!(file, "invalid: yaml: content: [[[").unwrap();

        let result = TasksFile::load_from(&tasks_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse tasks YAML file"));

        fs::remove_file(&tasks_path).unwrap();
    }

    #[test]
    fn test_total_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec![],
                },
            ],
        };

        assert_eq!(tasks_file.total_tasks(), 3);
    }

    #[test]
    fn test_completed_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: true,
                    depends: vec![],
                },
            ],
        };

        assert_eq!(tasks_file.completed_tasks(), 2);
    }

    #[test]
    fn test_remaining_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec![],
                },
            ],
        };

        assert_eq!(tasks_file.remaining_tasks(), 2);
    }

    #[test]
    fn test_incomplete_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec![],
                },
            ],
        };

        let incomplete = tasks_file.incomplete_tasks();
        assert_eq!(incomplete.len(), 2);
        assert_eq!(incomplete[0].title, "Task 2");
        assert_eq!(incomplete[1].title, "Task 3");
    }

    #[test]
    fn test_completed_task_list() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: true,
                    depends: vec![],
                },
            ],
        };

        let completed = tasks_file.completed_task_list();
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].title, "Task 1");
        assert_eq!(completed[1].title, "Task 3");
    }

    #[test]
    fn test_validate_unique_ids() {
        let yaml = r#"
tasks:
  - id: "task-1"
    title: "Task 1"
    completed: true
    depends: []
  - id: "task-1"
    title: "Task 1 Duplicate"
    completed: false
    depends: []
"#;

        let tasks_file: TasksFile = serde_yaml::from_str(yaml).unwrap();
        let result = tasks_file.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate task ID"));
    }

    #[test]
    fn test_validate_missing_dependency() {
        let yaml = r#"
tasks:
  - id: "task-1"
    title: "Task 1"
    completed: true
    depends: []
  - id: "task-2"
    title: "Task 2"
    completed: false
    depends: ["nonexistent-task"]
"#;

        let tasks_file: TasksFile = serde_yaml::from_str(yaml).unwrap();
        let result = tasks_file.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("depends on non-existent task"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let yaml = r#"
tasks:
  - id: "task-1"
    title: "Task 1"
    completed: false
    depends: ["task-2"]
  - id: "task-2"
    title: "Task 2"
    completed: false
    depends: ["task-1"]
"#;

        let tasks_file: TasksFile = serde_yaml::from_str(yaml).unwrap();
        let result = tasks_file.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_get_task_by_id() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec![],
                },
            ],
        };

        let task = tasks_file.get_task_by_id("task-1");
        assert!(task.is_some());
        assert_eq!(task.unwrap().title, "Task 1");

        let task = tasks_file.get_task_by_id("nonexistent");
        assert!(task.is_none());
    }

    #[test]
    fn test_get_ready_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: true,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string()],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec!["task-2".to_string()],
                },
                Task {
                    id: "task-4".to_string(),
                    title: "Task 4".to_string(),
                    completed: false,
                    depends: vec![],
                },
            ],
        };

        let ready = tasks_file.get_ready_tasks();
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|t| t.id == "task-2"));
        assert!(ready.iter().any(|t| t.id == "task-4"));
        assert!(!ready.iter().any(|t| t.id == "task-3"));
    }

    #[test]
    fn test_get_blocked_tasks() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string()],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string(), "task-2".to_string()],
                },
            ],
        };

        let blocked = tasks_file.get_blocked_tasks();
        assert_eq!(blocked.len(), 2);
        let (task2, deps2) = blocked.iter().find(|(t, _)| t.id == "task-2").unwrap();
        assert_eq!(task2.title, "Task 2");
        assert_eq!(deps2, &vec!["task-1".to_string()]);

        let (task3, deps3) = blocked.iter().find(|(t, _)| t.id == "task-3").unwrap();
        assert_eq!(task3.title, "Task 3");
        assert_eq!(deps3.len(), 2);
    }

    #[test]
    fn test_topological_order() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: false,
                    depends: vec![],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string()],
                },
                Task {
                    id: "task-3".to_string(),
                    title: "Task 3".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string(), "task-2".to_string()],
                },
            ],
        };

        let result = tasks_file.topological_order();
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.len(), 3);

        let task1_idx = order.iter().position(|t| t.id == "task-1").unwrap();
        let task2_idx = order.iter().position(|t| t.id == "task-2").unwrap();
        let task3_idx = order.iter().position(|t| t.id == "task-3").unwrap();

        assert!(task1_idx < task2_idx);
        assert!(task2_idx < task3_idx);
    }

    #[test]
    fn test_topological_order_circular() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    id: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    completed: false,
                    depends: vec!["task-2".to_string()],
                },
                Task {
                    id: "task-2".to_string(),
                    title: "Task 2".to_string(),
                    completed: false,
                    depends: vec!["task-1".to_string()],
                },
            ],
        };

        let result = tasks_file.topological_order();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }
}
