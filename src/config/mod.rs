use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
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
    #[serde(default)]
    pub overrides: Option<ProjectOverrides>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOverrides {
    pub max_iterations: Option<u32>,
    pub auto_pr: Option<bool>,
    pub draft_pr: Option<bool>,
}

impl ProjectConfig {
    pub fn effective_max_iterations(&self, global: &AgentConfig) -> u32 {
        self.overrides
            .as_ref()
            .and_then(|o| o.max_iterations)
            .unwrap_or(global.max_iterations)
    }

    pub fn effective_auto_pr(&self, global: &AgentConfig) -> bool {
        self.overrides
            .as_ref()
            .and_then(|o| o.auto_pr)
            .unwrap_or(global.auto_pr)
    }

    pub fn effective_draft_pr(&self, global: &AgentConfig) -> bool {
        self.overrides
            .as_ref()
            .and_then(|o| o.draft_pr)
            .unwrap_or(global.draft_pr)
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            ui: UiConfig::default(),
            agent: AgentConfig::default(),
            projects: Vec::new(),
        }
    }

    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory (XDG_CONFIG_HOME or ~/.config)")?;
        Ok(config_dir.join("lazyagent").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()
            .map_err(|e| anyhow::anyhow!("Config validation failed: {e}"))?;

        Ok(config)
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

            if let Some(overrides) = &project.overrides {
                if let Some(max_iter) = overrides.max_iterations {
                    if max_iter == 0 {
                        return Err(format!(
                            "Project '{}' override max_iterations must be greater than 0",
                            project.name
                        ));
                    }
                }
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
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_default_path_returns_valid_path() {
        let path = Config::default_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("lazyagent"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_load_from_missing_file() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let result = Config::load_from(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to read config file"));
    }

    #[test]
    fn test_load_from_invalid_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("invalid_config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "invalid toml content [[[").unwrap();

        let result = Config::load_from(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse config file"));

        fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_load_from_valid_config() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("valid_config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[ui]
refresh_ms = 300

[agent]
engine = "claude"
max_iterations = 5
auto_pr = false
draft_pr = true

[[projects]]
name = "test-project"
repo_path = "/absolute/path/to/repo"
tasks_yaml = "/absolute/path/to/tasks.yaml"
base_branch = "main"
max_parallel = 2
"#
        )
        .unwrap();

        let result = Config::load_from(&config_path);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.ui.refresh_ms, 300);
        assert_eq!(config.agent.max_iterations, 5);
        assert!(!config.agent.auto_pr);
        assert!(config.agent.draft_pr);
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "test-project");
        assert_eq!(config.projects[0].max_parallel, 2);

        fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_load_from_config_with_validation_error() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("invalid_validation_config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[ui]
refresh_ms = 200

[agent]
engine = "claude"
max_iterations = 3
auto_pr = true
draft_pr = false

[[projects]]
name = "test-project"
repo_path = "relative/path"
tasks_yaml = "/absolute/path/to/tasks.yaml"
base_branch = "main"
max_parallel = 3
"#
        )
        .unwrap();

        let result = Config::load_from(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Config validation failed"));
        assert!(err.to_string().contains("must be absolute"));

        fs::remove_file(&config_path).unwrap();
    }

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
            overrides: None,
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
            overrides: None,
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
            overrides: None,
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
            overrides: None,
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
            overrides: None,
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
            overrides: None,
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
            overrides: None,
        });
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_iterations must be greater than 0"));
    }

    #[test]
    fn test_project_overrides_inheritance() {
        let agent_config = AgentConfig {
            engine: "claude".to_string(),
            max_iterations: 3,
            auto_pr: true,
            draft_pr: false,
        };

        let project_no_override = ProjectConfig {
            name: "test-1".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
            overrides: None,
        };

        assert_eq!(project_no_override.effective_max_iterations(&agent_config), 3);
        assert!(project_no_override.effective_auto_pr(&agent_config));
        assert!(!project_no_override.effective_draft_pr(&agent_config));

        let project_with_override = ProjectConfig {
            name: "test-2".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
            overrides: Some(ProjectOverrides {
                max_iterations: Some(5),
                auto_pr: Some(false),
                draft_pr: None,
            }),
        };

        assert_eq!(project_with_override.effective_max_iterations(&agent_config), 5);
        assert!(!project_with_override.effective_auto_pr(&agent_config));
        assert!(!project_with_override.effective_draft_pr(&agent_config));
    }

    #[test]
    fn test_project_overrides_validation() {
        let mut config = Config::default();
        config.projects.push(ProjectConfig {
            name: "test".to_string(),
            repo_path: PathBuf::from("/absolute/path/to/repo"),
            tasks_yaml: PathBuf::from("/absolute/path/to/tasks.yaml"),
            base_branch: "main".to_string(),
            max_parallel: 3,
            overrides: Some(ProjectOverrides {
                max_iterations: Some(0),
                auto_pr: None,
                draft_pr: None,
            }),
        });

        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("override max_iterations must be greater than 0"));
    }

    #[test]
    fn test_load_config_with_overrides() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("config_with_overrides.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[ui]
refresh_ms = 200

[agent]
engine = "claude"
max_iterations = 3
auto_pr = true
draft_pr = false

[[projects]]
name = "project-1"
repo_path = "/absolute/path/to/repo1"
tasks_yaml = "/absolute/path/to/tasks1.yaml"
base_branch = "main"
max_parallel = 3

[[projects]]
name = "project-2"
repo_path = "/absolute/path/to/repo2"
tasks_yaml = "/absolute/path/to/tasks2.yaml"
base_branch = "develop"
max_parallel = 2

[projects.overrides]
max_iterations = 5
auto_pr = false
draft_pr = true
"#
        )
        .unwrap();

        let result = Config::load_from(&config_path);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.projects.len(), 2);

        assert!(config.projects[0].overrides.is_none());
        assert_eq!(config.projects[0].effective_max_iterations(&config.agent), 3);

        assert!(config.projects[1].overrides.is_some());
        assert_eq!(config.projects[1].effective_max_iterations(&config.agent), 5);
        assert!(!config.projects[1].effective_auto_pr(&config.agent));
        assert!(config.projects[1].effective_draft_pr(&config.agent));

        fs::remove_file(&config_path).unwrap();
    }
}
