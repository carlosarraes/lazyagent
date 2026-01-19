use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub title: String,
    pub completed: bool,
    #[serde(default)]
    pub parallel_group: Option<u32>,
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

    pub fn tasks_by_group(&self, group: u32) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.parallel_group == Some(group))
            .collect()
    }

    pub fn incomplete_tasks_by_group(&self, group: u32) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| !t.completed && t.parallel_group == Some(group))
            .collect()
    }

    pub fn next_parallel_group(&self) -> Option<u32> {
        self.tasks
            .iter()
            .filter(|t| !t.completed && t.parallel_group.is_some())
            .filter_map(|t| t.parallel_group)
            .min()
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
  - title: "Task 1"
    completed: true
    parallel_group: 1
  - title: "Task 2"
    completed: false
    parallel_group: 2
  - title: "Task 3"
    completed: false
"#;

        let tasks_file: TasksFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tasks_file.tasks.len(), 3);
        assert_eq!(tasks_file.tasks[0].title, "Task 1");
        assert!(tasks_file.tasks[0].completed);
        assert_eq!(tasks_file.tasks[0].parallel_group, Some(1));
        assert_eq!(tasks_file.tasks[1].title, "Task 2");
        assert!(!tasks_file.tasks[1].completed);
        assert_eq!(tasks_file.tasks[1].parallel_group, Some(2));
        assert_eq!(tasks_file.tasks[2].title, "Task 3");
        assert!(!tasks_file.tasks[2].completed);
        assert_eq!(tasks_file.tasks[2].parallel_group, None);
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
  - title: "Initialize project"
    completed: true
    parallel_group: 1
  - title: "Setup config"
    completed: false
    parallel_group: 2
"#
        )
        .unwrap();

        let result = TasksFile::load_from(&tasks_path);
        assert!(result.is_ok());
        let tasks_file = result.unwrap();
        assert_eq!(tasks_file.tasks.len(), 2);
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
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
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
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: true,
                    parallel_group: Some(2),
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
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
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
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
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
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: true,
                    parallel_group: Some(2),
                },
            ],
        };

        let completed = tasks_file.completed_task_list();
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].title, "Task 1");
        assert_eq!(completed[1].title, "Task 3");
    }

    #[test]
    fn test_tasks_by_group() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
                },
            ],
        };

        let group1 = tasks_file.tasks_by_group(1);
        assert_eq!(group1.len(), 2);
        assert_eq!(group1[0].title, "Task 1");
        assert_eq!(group1[1].title, "Task 2");

        let group2 = tasks_file.tasks_by_group(2);
        assert_eq!(group2.len(), 1);
        assert_eq!(group2[0].title, "Task 3");
    }

    #[test]
    fn test_incomplete_tasks_by_group() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
                },
            ],
        };

        let group1_incomplete = tasks_file.incomplete_tasks_by_group(1);
        assert_eq!(group1_incomplete.len(), 1);
        assert_eq!(group1_incomplete[0].title, "Task 2");

        let group2_incomplete = tasks_file.incomplete_tasks_by_group(2);
        assert_eq!(group2_incomplete.len(), 1);
        assert_eq!(group2_incomplete[0].title, "Task 3");
    }

    #[test]
    fn test_next_parallel_group() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: Some(3),
                },
                Task {
                    title: "Task 3".to_string(),
                    completed: false,
                    parallel_group: Some(2),
                },
            ],
        };

        assert_eq!(tasks_file.next_parallel_group(), Some(2));
    }

    #[test]
    fn test_next_parallel_group_all_complete() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    title: "Task 1".to_string(),
                    completed: true,
                    parallel_group: Some(1),
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: true,
                    parallel_group: Some(2),
                },
            ],
        };

        assert_eq!(tasks_file.next_parallel_group(), None);
    }

    #[test]
    fn test_next_parallel_group_no_groups() {
        let tasks_file = TasksFile {
            tasks: vec![
                Task {
                    title: "Task 1".to_string(),
                    completed: false,
                    parallel_group: None,
                },
                Task {
                    title: "Task 2".to_string(),
                    completed: false,
                    parallel_group: None,
                },
            ],
        };

        assert_eq!(tasks_file.next_parallel_group(), None);
    }
}
