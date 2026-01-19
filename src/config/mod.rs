use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    pub projects: Vec<ProjectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub refresh_ms: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { refresh_ms: 200 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub engine: String,
    pub max_iterations: u32,
    pub auto_pr: bool,
    pub draft_pr: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            engine: "claude".to_string(),
            max_iterations: 3,
            auto_pr: true,
            draft_pr: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub repo_path: PathBuf,
    pub tasks_yaml: PathBuf,
    pub base_branch: String,
    pub max_parallel: u32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            ui: UiConfig::default(),
            agent: AgentConfig::default(),
            projects: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.projects.is_empty() {
            return Err("At least one project must be configured".to_string());
        }

        for (idx, project) in self.projects.iter().enumerate() {
            if project.name.is_empty() {
                return Err(format!("Project {idx} has empty name"));
            }
            if !project.repo_path.is_absolute() {
                return Err(format!(
                    "Project '{}' repo_path must be absolute: {:?}",
                    project.name, project.repo_path
                ));
            }
            if project.base_branch.is_empty() {
                return Err(format!("Project '{}' has empty base_branch", project.name));
            }
            if project.max_parallel == 0 {
                return Err(format!(
                    "Project '{}' max_parallel must be greater than 0",
                    project.name
                ));
            }
        }

        if self.agent.engine != "claude" {
            return Err(format!(
                "Unsupported engine '{}', only 'claude' is supported",
                self.agent.engine
            ));
        }

        if self.agent.max_iterations == 0 {
            return Err("agent.max_iterations must be greater than 0".to_string());
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_config_defaults() {
        let ui = UiConfig::default();
        assert_eq!(ui.refresh_ms, 200);
    }

    #[test]
    fn test_agent_config_defaults() {
        let agent = AgentConfig::default();
        assert_eq!(agent.engine, "claude");
        assert_eq!(agent.max_iterations, 3);
        assert!(agent.auto_pr);
        assert!(!agent.draft_pr);
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.ui.refresh_ms, 200);
        assert_eq!(config.agent.engine, "claude");
        assert_eq!(config.projects.len(), 0);
    }

    #[test]
    fn test_config_validation_empty_projects() {
        let config = Config::default();
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "At least one project must be configured"
        );
    }

    #[test]
    fn test_config_validation_valid() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "test-project".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_name() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("has empty name"));
    }

    #[test]
    fn test_config_validation_relative_path() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("relative/path"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("must be absolute"));
    }

    #[test]
    fn test_config_validation_empty_branch() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("empty base_branch"));
    }

    #[test]
    fn test_config_validation_zero_max_parallel() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 0,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("must be greater than 0"));
    }

    #[test]
    fn test_config_validation_unsupported_engine() {
        let mut config = Config::default();
        config.agent.engine = "gpt-4".to_string();
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("Unsupported engine"));
    }

    #[test]
    fn test_config_validation_zero_max_iterations() {
        let mut config = Config::default();
        config.agent.max_iterations = 0;
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_iterations must be greater than 0"));
    }
}
