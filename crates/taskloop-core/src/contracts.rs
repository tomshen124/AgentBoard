use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    fs,
    path::{Path, PathBuf},
};

use crate::policy::{WorkspaceExecutionPolicy, DEFAULT_HEARTBEAT_INTERVAL_MS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownContractKind {
    Agents,
    Tools,
    Memory,
    Profile,
    Focus,
    Skill,
    Connector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownFrontmatter {
    pub kind: MarkdownContractKind,
    pub version: String,
    pub scope: String,
    pub owner: Option<String>,
    pub updated_at: Option<String>,
    pub source_of_truth: String,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownContract {
    pub path: PathBuf,
    pub frontmatter: MarkdownFrontmatter,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillMode {
    Simple,
    Complex,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMetadata {
    pub frontmatter: MarkdownFrontmatter,
    pub name: String,
    pub description: String,
    pub mode: SkillMode,
    pub work_modes: Vec<String>,
    pub tags: Vec<String>,
    pub safe_for_background: bool,
    pub requires_tools: Vec<String>,
    pub requires_env: Vec<String>,
    pub requires_models: Vec<String>,
    pub requires_os: Vec<String>,
    pub requires_bins: Vec<String>,
    pub requires_any_bins: Vec<String>,
    pub requires_config: Vec<String>,
    pub attachment_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSkill {
    pub directory_name: String,
    pub path: PathBuf,
    pub metadata: SkillMetadata,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceContracts {
    pub agents: Option<MarkdownContract>,
    pub tools: Option<MarkdownContract>,
    pub memory: Option<MarkdownContract>,
    pub profile: Option<MarkdownContract>,
    pub focus: Option<MarkdownContract>,
    pub skills: Vec<DiscoveredSkill>,
    pub execution_policy: WorkspaceExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderProtocol {
    OpenAiCompatible,
    AnthropicCompatible,
    Local,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiFamily {
    Responses,
    ChatCompletions,
    AnthropicMessages,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    Bearer,
    ApiKeyHeader,
    None,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderPolicy {
    Strict,
    Compatible,
    Extended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserAgentMode {
    RuntimeDefault,
    Custom(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderCapability {
    Streaming,
    ToolCalling,
    JsonOutput,
    ImageInput,
    Reasoning,
    WebSearch,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelAdapter {
    pub provider_id: String,
    pub base_url: String,
    pub protocol: ProviderProtocol,
    pub api_family: ApiFamily,
    pub auth: AuthMode,
    pub auth_header_name: Option<String>,
    pub header_policy: HeaderPolicy,
    pub user_agent_mode: UserAgentMode,
    pub static_headers: Vec<(String, String)>,
    pub models: Vec<String>,
    pub capabilities: Vec<ProviderCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorSyncMode {
    Inbound,
    Outbound,
    Bidirectional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    root: PathBuf,
}

impl WorkspaceLayout {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn agents_md(&self) -> PathBuf {
        self.root.join("AGENTS.md")
    }

    pub fn tools_md(&self) -> PathBuf {
        self.root.join("TOOLS.md")
    }

    pub fn memory_md(&self) -> PathBuf {
        self.root.join("MEMORY.md")
    }

    pub fn profile_md(&self) -> PathBuf {
        self.root.join("PROFILE.md")
    }

    pub fn focus_md(&self) -> PathBuf {
        self.root.join("FOCUS.md")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn skill_md(&self, skill_name: &str) -> PathBuf {
        self.skills_dir().join(skill_name).join("SKILL.md")
    }

    pub fn load_contracts(&self) -> Result<WorkspaceContracts, ContractLoadError> {
        let mut contracts = WorkspaceContracts {
            execution_policy: WorkspaceExecutionPolicy::default(),
            ..WorkspaceContracts::default()
        };

        contracts.agents = MarkdownContract::load_optional(self.agents_md())?;
        contracts.tools = MarkdownContract::load_optional(self.tools_md())?;
        contracts.memory = MarkdownContract::load_optional(self.memory_md())?;
        contracts.profile = MarkdownContract::load_optional(self.profile_md())?;
        contracts.focus = MarkdownContract::load_optional(self.focus_md())?;
        if let Some(agents) = &contracts.agents {
            contracts.execution_policy = WorkspaceExecutionPolicy::from_agents_contract(agents)?;
        }

        let skills_dir = self.skills_dir();
        if skills_dir.exists() {
            let mut skills = Vec::new();
            for entry in fs::read_dir(skills_dir)? {
                let entry = entry?;
                // Follow symlinked skill directories from external skill imports.
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                let directory_name = entry.file_name().to_string_lossy().to_string();
                let skill_path = entry_path.join("SKILL.md");
                if !skill_path.exists() {
                    continue;
                }

                skills.push(DiscoveredSkill::load(directory_name, skill_path)?);
            }
            skills.sort_by(|a, b| a.directory_name.cmp(&b.directory_name));
            contracts.skills = skills;
        }

        Ok(contracts)
    }
}

impl MarkdownContract {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ContractLoadError> {
        let path = path.into();
        let source = fs::read_to_string(&path)?;
        let (frontmatter, body) = parse_frontmatter(&source)?;
        Ok(Self {
            path,
            frontmatter: parse_common_frontmatter(&frontmatter)?,
            body,
        })
    }

    pub fn load_optional(path: impl Into<PathBuf>) -> Result<Option<Self>, ContractLoadError> {
        let path = path.into();
        if !path.exists() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }
}

impl DiscoveredSkill {
    pub fn load(directory_name: String, path: impl Into<PathBuf>) -> Result<Self, ContractLoadError> {
        let path = path.into();
        let source = fs::read_to_string(&path)?;
        let (frontmatter, body) = parse_frontmatter(&source)?;
        let common = parse_common_frontmatter(&frontmatter)?;
        let metadata = SkillMetadata {
            name: frontmatter_string(&frontmatter, "name")?,
            description: frontmatter_string(&frontmatter, "description")?,
            mode: parse_skill_mode(&frontmatter_string(&frontmatter, "mode")?)?,
            work_modes: frontmatter_list_or(&frontmatter, "modes", vec!["mixed".into()]),
            tags: frontmatter_list(&frontmatter, "tags"),
            safe_for_background: frontmatter_bool(&frontmatter, "safe_for_background")?,
            requires_tools: frontmatter_list(&frontmatter, "requires_tools"),
            requires_env: frontmatter_list(&frontmatter, "requires_env"),
            requires_models: frontmatter_list(&frontmatter, "requires_models"),
            requires_os: frontmatter_list(&frontmatter, "requires_os"),
            requires_bins: frontmatter_list(&frontmatter, "requires_bins"),
            requires_any_bins: frontmatter_list(&frontmatter, "requires_any_bins"),
            requires_config: frontmatter_list(&frontmatter, "requires_config"),
            attachment_types: frontmatter_list(&frontmatter, "attachment_types"),
            frontmatter: common,
        };

        Ok(Self {
            directory_name,
            path,
            metadata,
            body,
        })
    }
}

impl WorkspaceExecutionPolicy {
    pub fn from_agents_contract(contract: &MarkdownContract) -> Result<Self, ContractLoadError> {
        if !matches!(contract.frontmatter.kind, MarkdownContractKind::Agents) {
            return Err(ContractLoadError::InvalidField {
                field: "kind".into(),
                reason: "execution policy can only be derived from AGENTS.md".into(),
            });
        }

        let source = fs::read_to_string(&contract.path)?;
        let (frontmatter, _) = parse_frontmatter(&source)?;
        Ok(Self {
            writable_roots: match frontmatter.get("writable_roots") {
                Some(FrontmatterValue::List(values)) if !values.is_empty() => values.clone(),
                _ => vec![".".into()],
            },
            backup_before_write: frontmatter_bool_or(&frontmatter, "backup_before_write", true)?,
            destructive_requires_approval: frontmatter_bool_or(
                &frontmatter,
                "destructive_requires_approval",
                true,
            )?,
            exec_requires_approval: frontmatter_bool_or(&frontmatter, "exec_requires_approval", true)?,
            require_task_heartbeat: frontmatter_bool_or(
                &frontmatter,
                "require_task_heartbeat",
                true,
            )?,
            heartbeat_interval_ms: frontmatter_u64_or(
                &frontmatter,
                "heartbeat_interval_ms",
                DEFAULT_HEARTBEAT_INTERVAL_MS,
            )?,
            allowed_exec_languages: match frontmatter.get("allowed_exec_languages") {
                Some(FrontmatterValue::List(values)) if !values.is_empty() => values.clone(),
                _ => vec!["python".into(), "node".into(), "bash".into()],
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FrontmatterValue {
    String(String),
    Bool(bool),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractLoadError {
    Io(String),
    InvalidFormat(String),
    MissingField(String),
    InvalidField { field: String, reason: String },
}

impl fmt::Display for ContractLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "{message}"),
            Self::InvalidFormat(message) => write!(f, "{message}"),
            Self::MissingField(field) => write!(f, "missing required field `{field}`"),
            Self::InvalidField { field, reason } => {
                write!(f, "invalid field `{field}`: {reason}")
            }
        }
    }
}

impl Error for ContractLoadError {}

impl From<std::io::Error> for ContractLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

fn parse_frontmatter(source: &str) -> Result<(BTreeMap<String, FrontmatterValue>, String), ContractLoadError> {
    let normalized = source.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(ContractLoadError::InvalidFormat(
            "markdown contract must start with `---` frontmatter".to_string(),
        ));
    };
    let Some((frontmatter_block, body)) = rest.split_once("\n---\n") else {
        return Err(ContractLoadError::InvalidFormat(
            "markdown contract frontmatter must end with `---`".to_string(),
        ));
    };

    let lines: Vec<&str> = frontmatter_block.lines().collect();
    let mut index = 0usize;
    let mut data = BTreeMap::new();

    while index < lines.len() {
        let raw_line = lines[index];
        let line = raw_line.trim_end();
        index += 1;

        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with("  - ") || line.starts_with("- ") {
            return Err(ContractLoadError::InvalidFormat(
                "frontmatter list item is missing a parent key".to_string(),
            ));
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(ContractLoadError::InvalidFormat(format!(
                "frontmatter line is not a valid key/value pair: `{line}`"
            )));
        };

        let key = key.trim().to_string();
        let value = value.trim();
        if value.is_empty() {
            let mut items = Vec::new();
            while index < lines.len() {
                let next = lines[index].trim_end();
                let trimmed = next.trim_start();
                if let Some(item) = trimmed.strip_prefix("- ") {
                    items.push(strip_quotes(item.trim()));
                    index += 1;
                    continue;
                }
                break;
            }
            data.insert(key, FrontmatterValue::List(items));
            continue;
        }

        data.insert(key, parse_inline_value(value));
    }

    Ok((data, body.to_string()))
}

fn parse_inline_value(value: &str) -> FrontmatterValue {
    if value == "true" {
        return FrontmatterValue::Bool(true);
    }
    if value == "false" {
        return FrontmatterValue::Bool(false);
    }
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        if inner.trim().is_empty() {
            return FrontmatterValue::List(Vec::new());
        }
        let items = inner
            .split(',')
            .map(|item| strip_quotes(item.trim()))
            .collect::<Vec<_>>();
        return FrontmatterValue::List(items);
    }

    FrontmatterValue::String(strip_quotes(value))
}

fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let starts_and_ends_match = (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'');
        if starts_and_ends_match {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn parse_common_frontmatter(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
) -> Result<MarkdownFrontmatter, ContractLoadError> {
    Ok(MarkdownFrontmatter {
        kind: parse_contract_kind(&frontmatter_string(frontmatter, "kind")?)?,
        version: frontmatter_string(frontmatter, "version")?,
        scope: frontmatter_string(frontmatter, "scope")?,
        owner: frontmatter_optional_string(frontmatter, "owner"),
        updated_at: frontmatter_optional_string(frontmatter, "updated_at"),
        source_of_truth: frontmatter_string(frontmatter, "source_of_truth")?,
        visibility: frontmatter_string(frontmatter, "visibility")?,
    })
}

fn parse_contract_kind(value: &str) -> Result<MarkdownContractKind, ContractLoadError> {
    match value {
        "agents" => Ok(MarkdownContractKind::Agents),
        "tools" => Ok(MarkdownContractKind::Tools),
        "memory" => Ok(MarkdownContractKind::Memory),
        "profile" => Ok(MarkdownContractKind::Profile),
        "focus" => Ok(MarkdownContractKind::Focus),
        "skill" => Ok(MarkdownContractKind::Skill),
        "connector" => Ok(MarkdownContractKind::Connector),
        other => Err(ContractLoadError::InvalidField {
            field: "kind".to_string(),
            reason: format!("unsupported kind `{other}`"),
        }),
    }
}

fn parse_skill_mode(value: &str) -> Result<SkillMode, ContractLoadError> {
    match value {
        "simple" => Ok(SkillMode::Simple),
        "complex" => Ok(SkillMode::Complex),
        "both" => Ok(SkillMode::Both),
        other => Err(ContractLoadError::InvalidField {
            field: "mode".to_string(),
            reason: format!("unsupported skill mode `{other}`"),
        }),
    }
}

fn frontmatter_string(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
) -> Result<String, ContractLoadError> {
    match frontmatter.get(key) {
        Some(FrontmatterValue::String(value)) => Ok(value.clone()),
        Some(FrontmatterValue::Bool(_)) | Some(FrontmatterValue::List(_)) => {
            Err(ContractLoadError::InvalidField {
                field: key.to_string(),
                reason: "expected string".to_string(),
            })
        }
        None => Err(ContractLoadError::MissingField(key.to_string())),
    }
}

fn frontmatter_optional_string(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
) -> Option<String> {
    match frontmatter.get(key) {
        Some(FrontmatterValue::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn frontmatter_bool(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
) -> Result<bool, ContractLoadError> {
    match frontmatter.get(key) {
        Some(FrontmatterValue::Bool(value)) => Ok(*value),
        Some(FrontmatterValue::String(value)) => match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ContractLoadError::InvalidField {
                field: key.to_string(),
                reason: "expected boolean".to_string(),
            }),
        },
        Some(FrontmatterValue::List(_)) => Err(ContractLoadError::InvalidField {
            field: key.to_string(),
            reason: "expected boolean".to_string(),
        }),
        None => Err(ContractLoadError::MissingField(key.to_string())),
    }
}

fn frontmatter_bool_or(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
    default: bool,
) -> Result<bool, ContractLoadError> {
    match frontmatter.get(key) {
        Some(_) => frontmatter_bool(frontmatter, key),
        None => Ok(default),
    }
}

fn frontmatter_u64_or(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
    default: u64,
) -> Result<u64, ContractLoadError> {
    match frontmatter.get(key) {
        Some(FrontmatterValue::String(value)) => value.parse::<u64>().map_err(|_| {
            ContractLoadError::InvalidField {
                field: key.to_string(),
                reason: "expected unsigned integer".into(),
            }
        }),
        Some(FrontmatterValue::Bool(_)) | Some(FrontmatterValue::List(_)) => {
            Err(ContractLoadError::InvalidField {
                field: key.to_string(),
                reason: "expected unsigned integer".into(),
            })
        }
        None => Ok(default),
    }
}

fn frontmatter_list(frontmatter: &BTreeMap<String, FrontmatterValue>, key: &str) -> Vec<String> {
    match frontmatter.get(key) {
        Some(FrontmatterValue::List(values)) => values.clone(),
        Some(FrontmatterValue::String(value)) if value.is_empty() => Vec::new(),
        _ => Vec::new(),
    }
}

fn frontmatter_list_or(
    frontmatter: &BTreeMap<String, FrontmatterValue>,
    key: &str,
    default: Vec<String>,
) -> Vec<String> {
    let values = frontmatter_list(frontmatter, key);
    if values.is_empty() {
        default
    } else {
        values
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{MarkdownContract, SkillMode, WorkspaceExecutionPolicy, WorkspaceLayout};

    #[test]
    fn workspace_layout_points_to_expected_contract_files() {
        let layout = WorkspaceLayout::new("/tmp/agentboard");
        assert_eq!(layout.agents_md().to_string_lossy(), "/tmp/agentboard/AGENTS.md");
        assert_eq!(layout.profile_md().to_string_lossy(), "/tmp/agentboard/PROFILE.md");
        assert_eq!(layout.focus_md().to_string_lossy(), "/tmp/agentboard/FOCUS.md");
        assert_eq!(
            layout.skill_md("example_skill").to_string_lossy(),
            "/tmp/agentboard/skills/example_skill/SKILL.md"
        );
    }

    #[test]
    fn markdown_contract_loader_reads_frontmatter_and_body() {
        let temp_dir = create_temp_workspace("contract_loader");
        let path = temp_dir.join("AGENTS.md");
        fs::write(
            &path,
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

# Rules

- Keep tasks observable
"#,
        )
        .unwrap();

        let contract = MarkdownContract::load(&path).unwrap();
        assert!(matches!(
            contract.frontmatter.kind,
            super::MarkdownContractKind::Agents
        ));
        assert!(contract.body.contains("Keep tasks observable"));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn workspace_loader_discovers_skills() {
        let temp_dir = create_temp_workspace("skill_loader");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("skills").join("demo")).unwrap();
        fs::write(
            temp_dir.join("PROFILE.md"),
            r#"---
kind: profile
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: user_editable_contract
visibility: internal
---

Work with crisp technical judgment.
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("FOCUS.md"),
            r#"---
kind: focus
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: user_editable_contract
visibility: internal
---

Current focus goes here.
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("skills").join("demo").join("SKILL.md"),
            r#"---
kind: skill
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
name: Demo Skill
description: Test loader
mode: both
safe_for_background: false
requires_tools:
  - exec
requires_env: []
requires_models: []
requires_os: []
requires_bins:
  - git
requires_any_bins: []
requires_config: []
attachment_types:
  - file
modes:
  - coding
tags:
  - demo
---

Demo body
"#,
        )
        .unwrap();

        let layout = WorkspaceLayout::new(&temp_dir);
        let contracts = layout.load_contracts().unwrap();
        assert!(contracts.agents.is_some());
        assert!(contracts.profile.is_some());
        assert!(contracts.focus.is_some());
        assert_eq!(contracts.skills.len(), 1);
        assert_eq!(contracts.skills[0].metadata.name, "Demo Skill");
        assert_eq!(contracts.skills[0].metadata.mode, SkillMode::Both);
        assert_eq!(contracts.skills[0].metadata.work_modes, vec!["coding"]);
        assert_eq!(contracts.skills[0].metadata.tags, vec!["demo"]);
        assert_eq!(contracts.skills[0].metadata.requires_tools, vec!["exec"]);
        assert_eq!(contracts.skills[0].metadata.requires_bins, vec!["git"]);
        assert_eq!(contracts.skills[0].metadata.attachment_types, vec!["file"]);
        assert!(contracts.execution_policy.backup_before_write);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn execution_policy_reads_agents_frontmatter() {
        let temp_dir = create_temp_workspace("agents-policy");
        let path = temp_dir.join("AGENTS.md");
        fs::write(
            &path,
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - .
  - docs
backup_before_write: true
destructive_requires_approval: true
exec_requires_approval: false
require_task_heartbeat: true
heartbeat_interval_ms: 15000
allowed_exec_languages:
  - python
  - bash
---

Policy body
"#,
        )
        .unwrap();

        let contract = MarkdownContract::load(&path).unwrap();
        let policy = WorkspaceExecutionPolicy::from_agents_contract(&contract).unwrap();
        assert_eq!(policy.writable_roots, vec![".", "docs"]);
        assert!(!policy.exec_requires_approval);
        assert_eq!(policy.heartbeat_interval_ms, 15_000);
        assert_eq!(policy.allowed_exec_languages, vec!["python", "bash"]);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    fn create_temp_workspace(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agentboard-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
