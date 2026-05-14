use std::collections::BTreeMap;

use crate::{
    contracts::{DiscoveredSkill, SkillMode, WorkspaceContracts},
    skill_gating::{evaluate_load_time_gating, GatingContext},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionKind {
    Native,
    Exec,
    ConnectorBacked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub id: String,
    pub description: String,
    pub execution_kind: ToolExecutionKind,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        for tool in [
            ToolDefinition {
                id: "filesystem".into(),
                description: "Inspect, search, read, write, copy, move, and remove workspace files".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "exec".into(),
                description: "Run CLI commands and scripts".into(),
                execution_kind: ToolExecutionKind::Exec,
                requires_approval: true,
            },
            ToolDefinition {
                id: "shell".into(),
                description: "Run shell-oriented project automation and command sequences".into(),
                execution_kind: ToolExecutionKind::Exec,
                requires_approval: true,
            },
            ToolDefinition {
                id: "http".into(),
                description: "Perform outbound HTTP requests".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "web_search".into(),
                description: "Search the web through a configured search provider".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "web_extract".into(),
                description: "Extract and normalize webpage content into markdown or text".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "browser".into(),
                description: "Drive a browser or web page session".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "memory".into(),
                description: "Read and write structured memory".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "git".into(),
                description: "Inspect and operate on the local git repository".into(),
                execution_kind: ToolExecutionKind::Exec,
                requires_approval: true,
            },
            ToolDefinition {
                id: "archive".into(),
                description: "Pack and unpack zip, tar, and other archive formats".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "desktop".into(),
                description: "Open, reveal, and hand off workspace paths to the desktop shell".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
            ToolDefinition {
                id: "cron".into(),
                description: "Register scheduled patrol or background wakeups".into(),
                execution_kind: ToolExecutionKind::Native,
                requires_approval: false,
            },
        ] {
            registry.register(tool);
        }
        registry
    }

    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.id.clone(), tool);
    }

    pub fn get(&self, tool_id: &str) -> Option<&ToolDefinition> {
        self.tools.get(tool_id)
    }

    pub fn all(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisteredSkillSource {
    BuiltIn,
    Workspace,
    LocalLinked,
    Marketplace,
    RemoteGit,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: SkillMode,
    pub work_modes: Vec<String>,
    pub tags: Vec<String>,
    pub requires_tools: Vec<String>,
    pub requires_env: Vec<String>,
    pub requires_models: Vec<String>,
    pub requires_os: Vec<String>,
    pub requires_bins: Vec<String>,
    pub requires_any_bins: Vec<String>,
    pub requires_config: Vec<String>,
    pub attachment_types: Vec<String>,
    pub safe_for_background: bool,
    pub enabled: bool,
    pub gating_status: String,
    pub gating_reasons: Vec<String>,
    pub path: String,
    pub source: RegisteredSkillSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillRegistry {
    skills: BTreeMap<String, RegisteredSkill>,
}

impl SkillRegistry {
    pub fn with_builtin_defaults() -> Self {
        let mut registry = Self::default();
        for skill in builtin_skills() {
            registry.skills.insert(skill.id.clone(), skill);
        }
        registry
    }

    pub fn from_workspace_contracts(contracts: &WorkspaceContracts) -> Self {
        let gating_context = GatingContext::detect();
        let mut registry = Self::with_builtin_defaults();
        for skill in &contracts.skills {
            registry.register_with_gating(skill, &gating_context);
        }
        registry
    }

    pub fn register(&mut self, skill: &DiscoveredSkill) {
        self.register_with_gating(skill, &GatingContext::detect());
    }

    pub fn register_with_gating(&mut self, skill: &DiscoveredSkill, ctx: &GatingContext) {
        let mut registered = RegisteredSkill {
            id: skill.directory_name.clone(),
            name: skill.metadata.name.clone(),
            description: skill.metadata.description.clone(),
            mode: skill.metadata.mode.clone(),
            work_modes: skill.metadata.work_modes.clone(),
            tags: skill.metadata.tags.clone(),
            requires_tools: skill.metadata.requires_tools.clone(),
            requires_env: skill.metadata.requires_env.clone(),
            requires_models: skill.metadata.requires_models.clone(),
            requires_os: skill.metadata.requires_os.clone(),
            requires_bins: skill.metadata.requires_bins.clone(),
            requires_any_bins: skill.metadata.requires_any_bins.clone(),
            requires_config: skill.metadata.requires_config.clone(),
            attachment_types: skill.metadata.attachment_types.clone(),
            safe_for_background: skill.metadata.safe_for_background,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: skill.path.to_string_lossy().to_string(),
            source: RegisteredSkillSource::Workspace,
        };
        let gating = evaluate_load_time_gating(&registered, ctx);
        registered.enabled = gating.enabled;
        registered.gating_status = gating.status;
        registered.gating_reasons = gating.reasons;
        self.skills.insert(registered.id.clone(), registered);
    }

    pub fn register_enabled(&mut self, skill: &DiscoveredSkill) {
        let registered = RegisteredSkill {
            id: skill.directory_name.clone(),
            name: skill.metadata.name.clone(),
            description: skill.metadata.description.clone(),
            mode: skill.metadata.mode.clone(),
            work_modes: skill.metadata.work_modes.clone(),
            tags: skill.metadata.tags.clone(),
            requires_tools: skill.metadata.requires_tools.clone(),
            requires_env: skill.metadata.requires_env.clone(),
            requires_models: skill.metadata.requires_models.clone(),
            requires_os: skill.metadata.requires_os.clone(),
            requires_bins: skill.metadata.requires_bins.clone(),
            requires_any_bins: skill.metadata.requires_any_bins.clone(),
            requires_config: skill.metadata.requires_config.clone(),
            attachment_types: skill.metadata.attachment_types.clone(),
            safe_for_background: skill.metadata.safe_for_background,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: skill.path.to_string_lossy().to_string(),
            source: RegisteredSkillSource::Workspace,
        };
        self.skills.insert(registered.id.clone(), registered);
    }

    pub fn get(&self, skill_id: &str) -> Option<&RegisteredSkill> {
        self.skills.get(skill_id)
    }

    pub fn all(&self) -> Vec<&RegisteredSkill> {
        self.skills.values().collect()
    }
}

fn builtin_skills() -> [RegisteredSkill; 7] {
    [
        RegisteredSkill {
            id: "workspace_maintenance".into(),
            name: "Workspace Maintenance".into(),
            description: "Maintain workspace rules, contracts, and safe local operations".into(),
            mode: SkillMode::Both,
            work_modes: vec!["mixed".into()],
            tags: vec!["workspace".into(), "maintenance".into()],
            requires_tools: vec!["filesystem".into(), "shell".into(), "memory".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: Vec::new(),
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://workspace_maintenance".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "document_writer".into(),
            name: "Document Writer".into(),
            description: "Draft and update markdown or document artifacts inside the workspace".into(),
            mode: SkillMode::Both,
            work_modes: vec!["daily".into(), "mixed".into()],
            tags: vec!["document".into(), "artifact".into()],
            requires_tools: vec!["filesystem".into(), "memory".into(), "desktop".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: vec!["file".into()],
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://document_writer".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "background_patrol".into(),
            name: "Background Patrol".into(),
            description: "Run patrol-style checks, scheduled follow-ups, and background wakeups".into(),
            mode: SkillMode::Complex,
            work_modes: vec!["daily".into(), "mixed".into()],
            tags: vec!["automation".into(), "schedule".into()],
            requires_tools: vec!["cron".into(), "memory".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: Vec::new(),
            safe_for_background: true,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://background_patrol".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "repo_reader".into(),
            name: "Repo Reader".into(),
            description: "Read repository structure, inspect diffs, and summarize codebase context".into(),
            mode: SkillMode::Both,
            work_modes: vec!["coding".into(), "mixed".into()],
            tags: vec!["code".into(), "repo".into()],
            requires_tools: vec!["filesystem".into(), "git".into(), "shell".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: vec!["git".into()],
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: Vec::new(),
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://repo_reader".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "artifact_exporter".into(),
            name: "Artifact Exporter".into(),
            description: "Prepare exportable artifacts and deliver path-first outputs inside the workspace".into(),
            mode: SkillMode::Both,
            work_modes: vec!["daily".into(), "mixed".into()],
            tags: vec!["artifact".into(), "export".into()],
            requires_tools: vec!["filesystem".into(), "archive".into(), "desktop".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: vec!["file".into()],
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://artifact_exporter".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "research_brief".into(),
            name: "Research Brief".into(),
            description: "Search, extract, and condense external references into a local brief".into(),
            mode: SkillMode::Both,
            work_modes: vec!["daily".into(), "mixed".into()],
            tags: vec!["research".into(), "web".into()],
            requires_tools: vec!["web_search".into(), "web_extract".into(), "filesystem".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: Vec::new(),
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://research_brief".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
        RegisteredSkill {
            id: "project_init".into(),
            name: "Project Init".into(),
            description: "Initialize workspace contracts, starter skills, and first-run project scaffolding".into(),
            mode: SkillMode::Both,
            work_modes: vec!["mixed".into()],
            tags: vec!["workspace".into(), "setup".into()],
            requires_tools: vec!["filesystem".into(), "shell".into(), "memory".into()],
            requires_env: Vec::new(),
            requires_models: Vec::new(),
            requires_os: Vec::new(),
            requires_bins: Vec::new(),
            requires_any_bins: Vec::new(),
            requires_config: Vec::new(),
            attachment_types: Vec::new(),
            safe_for_background: false,
            enabled: true,
            gating_status: "available".into(),
            gating_reasons: Vec::new(),
            path: "builtin://project_init".into(),
            source: RegisteredSkillSource::BuiltIn,
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::contracts::{
        DiscoveredSkill, MarkdownContractKind, MarkdownFrontmatter, SkillMetadata, SkillMode,
        WorkspaceContracts,
    };
    use crate::policy::WorkspaceExecutionPolicy;

    use super::{RegisteredSkillSource, SkillRegistry, ToolRegistry};

    #[test]
    fn default_tool_registry_contains_exec_and_filesystem() {
        let registry = ToolRegistry::with_defaults();
        assert!(registry.get("exec").is_some());
        assert!(registry.get("shell").is_some());
        assert!(registry.get("filesystem").is_some());
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("git").is_some());
    }

    #[test]
    fn skill_registry_builds_from_workspace_contracts() {
        let contracts = WorkspaceContracts {
            agents: None,
            tools: None,
            memory: None,
            profile: None,
            focus: None,
            skills: vec![DiscoveredSkill {
                directory_name: "demo".into(),
                path: "/tmp/demo/SKILL.md".into(),
                metadata: SkillMetadata {
                    frontmatter: MarkdownFrontmatter {
                        kind: MarkdownContractKind::Skill,
                        version: "1".into(),
                        scope: "workspace".into(),
                        owner: None,
                        updated_at: Some("2026-03-25".into()),
                        source_of_truth: "markdown_contract".into(),
                        visibility: "internal".into(),
                    },
                    name: "Demo".into(),
                    description: "Demo skill".into(),
                    mode: SkillMode::Both,
                    work_modes: vec!["coding".into()],
                    tags: vec!["demo".into()],
                    safe_for_background: false,
                    requires_tools: vec!["exec".into()],
                    requires_env: Vec::new(),
                    requires_models: Vec::new(),
                    requires_os: Vec::new(),
                    requires_bins: vec!["definitely-missing-agentboard-bin".into()],
                    requires_any_bins: Vec::new(),
                    requires_config: Vec::new(),
                    attachment_types: vec!["file".into()],
                },
                body: "body".into(),
            }],
            execution_policy: WorkspaceExecutionPolicy::default(),
        };

        let registry = SkillRegistry::from_workspace_contracts(&contracts);
        assert!(registry.get("workspace_maintenance").is_some());
        let skill = registry.get("demo").unwrap();
        assert_eq!(skill.name, "Demo");
        assert_eq!(skill.work_modes, vec!["coding"]);
        assert_eq!(skill.tags, vec!["demo"]);
        assert_eq!(skill.requires_tools, vec!["exec"]);
        assert_eq!(skill.requires_env, Vec::<String>::new());
        assert_eq!(skill.source, RegisteredSkillSource::Workspace);
        assert!(!skill.enabled);
        assert_eq!(skill.gating_status, "disabled");
        assert!(skill
            .gating_reasons
            .iter()
            .any(|reason| reason.contains("definitely-missing-agentboard-bin")));
    }

    #[test]
    fn builtin_skill_registry_contains_default_skills() {
        let registry = SkillRegistry::with_builtin_defaults();
        let patrol = registry.get("background_patrol").unwrap();
        assert!(patrol.safe_for_background);
        assert!(patrol.enabled);
        assert_eq!(patrol.gating_status, "available");
        assert_eq!(patrol.source, RegisteredSkillSource::BuiltIn);
        assert!(registry.get("repo_reader").is_some());
        assert!(registry.get("research_brief").is_some());
        assert!(registry.get("project_init").is_some());
    }
}
