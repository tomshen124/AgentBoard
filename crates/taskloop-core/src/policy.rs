use std::path::{Component, Path, PathBuf};
use serde::{Deserialize, Serialize};

pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow,
    AllowWithGuard,
    RequireApproval,
    Deny,
    DenyAndReplan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRiskLevel {
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceExecutionPolicy {
    pub writable_roots: Vec<String>,
    pub backup_before_write: bool,
    pub destructive_requires_approval: bool,
    pub exec_requires_approval: bool,
    pub require_task_heartbeat: bool,
    pub heartbeat_interval_ms: u64,
    pub allowed_exec_languages: Vec<String>,
}

impl Default for WorkspaceExecutionPolicy {
    fn default() -> Self {
        Self {
            writable_roots: vec![".".into()],
            backup_before_write: true,
            destructive_requires_approval: true,
            exec_requires_approval: false,
            require_task_heartbeat: true,
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            allowed_exec_languages: vec!["python".into(), "node".into(), "bash".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecPlan {
    pub decision: PermissionDecision,
    pub allowed: bool,
    pub requires_approval: bool,
    pub risk_level: ApprovalRiskLevel,
    pub interpreter: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWritePlan {
    pub decision: PermissionDecision,
    pub allowed: bool,
    pub requires_approval: bool,
    pub risk_level: ApprovalRiskLevel,
    pub create_backup: bool,
    pub normalized_path: PathBuf,
    pub reason: String,
}

impl WorkspaceExecutionPolicy {
    pub fn evaluate_exec_command(&self, command: &str) -> ExecPlan {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return ExecPlan {
                decision: PermissionDecision::Deny,
                allowed: false,
                requires_approval: false,
                risk_level: ApprovalRiskLevel::High,
                interpreter: None,
                reason: "empty command".into(),
            };
        }

        let interpreter = trimmed
            .split_whitespace()
            .next()
            .map(normalize_exec_language);

        if let Some(interpreter_name) = interpreter.clone() {
            if self.allowed_exec_languages.iter().any(|item| item == &interpreter_name) {
                let high_risk = command_requires_high_risk_approval(trimmed);
                let requires_approval = self.exec_requires_approval || high_risk;
                return ExecPlan {
                    decision: if requires_approval {
                        PermissionDecision::RequireApproval
                    } else {
                        PermissionDecision::Allow
                    },
                    allowed: true,
                    requires_approval,
                    risk_level: if high_risk {
                        ApprovalRiskLevel::High
                    } else {
                        ApprovalRiskLevel::Low
                    },
                    interpreter: Some(interpreter_name),
                    reason: if high_risk {
                        "allowed interpreter but high-risk command pattern detected".into()
                    } else {
                        "command matches allowed interpreter and low-risk policy".into()
                    },
                };
            }
        }

        ExecPlan {
            decision: PermissionDecision::DenyAndReplan,
            allowed: false,
            requires_approval: true,
            risk_level: ApprovalRiskLevel::High,
            interpreter,
            reason: "command interpreter is outside the workspace policy allowlist".into(),
        }
    }

    pub fn evaluate_file_write(
        &self,
        workspace_root: &Path,
        target_path: &Path,
        destructive: bool,
    ) -> FileWritePlan {
        let normalized_path = normalize_target_path(workspace_root, target_path);
        let allowed = self
            .writable_roots
            .iter()
            .map(|item| normalize_target_path(workspace_root, Path::new(item)))
            .any(|root| path_is_within(&normalized_path, &root));

        if !allowed {
            return FileWritePlan {
                decision: PermissionDecision::Deny,
                allowed: false,
                requires_approval: true,
                risk_level: ApprovalRiskLevel::High,
                create_backup: false,
                normalized_path,
                reason: "target path is outside configured writable roots".into(),
            };
        }

        FileWritePlan {
            decision: if destructive && self.destructive_requires_approval {
                PermissionDecision::RequireApproval
            } else if self.backup_before_write {
                PermissionDecision::AllowWithGuard
            } else {
                PermissionDecision::Allow
            },
            allowed: true,
            requires_approval: destructive && self.destructive_requires_approval,
            risk_level: if destructive {
                ApprovalRiskLevel::High
            } else {
                ApprovalRiskLevel::Low
            },
            create_backup: self.backup_before_write,
            normalized_path,
            reason: if destructive {
                "destructive write allowed but may require approval".into()
            } else {
                "write allowed inside workspace policy roots".into()
            },
        }
    }
}

fn normalize_exec_language(token: &str) -> String {
    let lowered = Path::new(token)
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| token.to_ascii_lowercase());

    if lowered.starts_with("python") {
        "python".into()
    } else if lowered.starts_with("node") {
        "node".into()
    } else if lowered == "sh" || lowered == "zsh" || lowered == "bash" {
        "bash".into()
    } else {
        lowered
    }
}

fn command_requires_high_risk_approval(command: &str) -> bool {
    let lowered = command.trim().to_ascii_lowercase();
    let destructive_patterns = [
        " rm ",
        "rm -",
        " rm-",
        "sudo ",
        "git reset --hard",
        "git clean -fd",
        "git clean -xdf",
        "chmod ",
        "chown ",
        "kill ",
        "launchctl ",
        "shutdown ",
        "reboot ",
        "diskutil erase",
        "mkfs",
        " dd ",
    ];

    let inline_exec = lowered.starts_with("python -c")
        || lowered.starts_with("python3 -c")
        || lowered.starts_with("node -e");

    inline_exec || destructive_patterns.iter().any(|pattern| lowered.contains(pattern))
}

fn normalize_target_path(workspace_root: &Path, target_path: &Path) -> PathBuf {
    let joined = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        workspace_root.join(target_path)
    };
    lexical_normalize(joined)
}

fn lexical_normalize(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ApprovalRiskLevel, PermissionDecision, WorkspaceExecutionPolicy};

    #[test]
    fn exec_policy_allows_python_when_listed() {
        let policy = WorkspaceExecutionPolicy::default();
        let plan = policy.evaluate_exec_command("python script.py");
        assert!(plan.allowed);
        assert_eq!(plan.decision, PermissionDecision::Allow);
        assert_eq!(plan.risk_level, ApprovalRiskLevel::Low);
        assert_eq!(plan.interpreter.as_deref(), Some("python"));
    }

    #[test]
    fn exec_policy_requires_approval_for_high_risk_command_patterns() {
        let policy = WorkspaceExecutionPolicy::default();
        let plan = policy.evaluate_exec_command("bash -lc rm -rf build");
        assert!(plan.allowed);
        assert_eq!(plan.decision, PermissionDecision::RequireApproval);
        assert!(plan.requires_approval);
        assert_eq!(plan.risk_level, ApprovalRiskLevel::High);
    }

    #[test]
    fn exec_policy_blocks_unknown_interpreter() {
        let policy = WorkspaceExecutionPolicy::default();
        let plan = policy.evaluate_exec_command("ruby script.rb");
        assert!(!plan.allowed);
        assert_eq!(plan.decision, PermissionDecision::DenyAndReplan);
    }

    #[test]
    fn file_write_policy_restricts_paths_outside_roots() {
        let policy = WorkspaceExecutionPolicy::default();
        let plan = policy.evaluate_file_write(
            Path::new("/tmp/workspace"),
            Path::new("../outside.txt"),
            false,
        );
        assert!(!plan.allowed);
        assert_eq!(plan.decision, PermissionDecision::Deny);
        assert_eq!(plan.risk_level, ApprovalRiskLevel::High);
    }
}
