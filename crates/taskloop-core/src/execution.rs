use std::{
    error::Error,
    fmt,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use serde::{Deserialize, Serialize};

use crate::policy::{ExecPlan, FileWritePlan, PermissionDecision};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub approval_granted: bool,
}

impl ExecRequest {
    pub fn command_preview(&self) -> String {
        let mut parts = vec![self.program.clone()];
        parts.extend(self.args.clone());
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecResult {
    pub plan: ExecPlan,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWriteRequest {
    pub path: PathBuf,
    pub content: Vec<u8>,
    pub approval_granted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWriteResult {
    pub plan: FileWritePlan,
    pub bytes_written: usize,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeActionError {
    UnknownTask(String),
    PolicyDenied { reason: String },
    ApprovalRequired { reason: String },
    ReplanSuggested { reason: String },
    InvalidResumeCheckpoint { reason: String },
    InvalidConnectorHostKey { reason: String },
    UnsupportedClientAction { action_key: String },
    MissingActionReason { action_key: String },
    InvalidWorkingDirectory { reason: String },
    Io(String),
}

impl fmt::Display for RuntimeActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTask(task_id) => write!(f, "unknown task `{task_id}`"),
            Self::PolicyDenied { reason } => write!(f, "policy denied action: {reason}"),
            Self::ApprovalRequired { reason } => write!(f, "approval required: {reason}"),
            Self::ReplanSuggested { reason } => write!(f, "replan suggested: {reason}"),
            Self::InvalidResumeCheckpoint { reason } => {
                write!(f, "invalid resume checkpoint: {reason}")
            }
            Self::InvalidConnectorHostKey { reason } => {
                write!(f, "invalid connector host key: {reason}")
            }
            Self::UnsupportedClientAction { action_key } => {
                write!(f, "unsupported client action `{action_key}`")
            }
            Self::MissingActionReason { action_key } => {
                write!(f, "missing required reason for client action `{action_key}`")
            }
            Self::InvalidWorkingDirectory { reason } => {
                write!(f, "invalid working directory: {reason}")
            }
            Self::Io(reason) => write!(f, "{reason}"),
        }
    }
}

impl Error for RuntimeActionError {}

impl From<std::io::Error> for RuntimeActionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub(crate) fn run_exec(
    request: ExecRequest,
    plan: ExecPlan,
    workspace_root: &Path,
) -> Result<ExecResult, RuntimeActionError> {
    let command_preview = request.command_preview();
    match plan.decision {
        PermissionDecision::Deny => {
            return Err(RuntimeActionError::PolicyDenied {
                reason: plan.reason.clone(),
            })
        }
        PermissionDecision::DenyAndReplan => {
            return Err(RuntimeActionError::ReplanSuggested {
                reason: plan.reason.clone(),
            })
        }
        PermissionDecision::RequireApproval if !request.approval_granted => {
            return Err(RuntimeActionError::ApprovalRequired {
                reason: format!("{} (command: {})", plan.reason, command_preview),
            })
        }
        _ => {}
    }
    if plan.requires_approval && !request.approval_granted {
        return Err(RuntimeActionError::ApprovalRequired {
            reason: format!("{} (command: {})", plan.reason, command_preview),
        });
    }

    let cwd = request
        .cwd
        .clone()
        .unwrap_or_else(|| workspace_root.to_path_buf());
    if !cwd.starts_with(workspace_root) {
        return Err(RuntimeActionError::InvalidWorkingDirectory {
            reason: "cwd must stay inside workspace root".into(),
        });
    }

    let output = Command::new(&request.program)
        .args(&request.args)
        .current_dir(&cwd)
        .output()?;

    Ok(ExecResult {
        plan,
        command: command_preview,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        cwd,
    })
}

pub(crate) fn run_file_write(
    request: FileWriteRequest,
    plan: FileWritePlan,
) -> Result<FileWriteResult, RuntimeActionError> {
    let path_preview = plan.normalized_path.display().to_string();
    match plan.decision {
        PermissionDecision::Deny => {
            return Err(RuntimeActionError::PolicyDenied {
                reason: plan.reason.clone(),
            })
        }
        PermissionDecision::DenyAndReplan => {
            return Err(RuntimeActionError::ReplanSuggested {
                reason: plan.reason.clone(),
            })
        }
        PermissionDecision::RequireApproval if !request.approval_granted => {
            return Err(RuntimeActionError::ApprovalRequired {
                reason: format!("{} (path: {})", plan.reason, path_preview),
            })
        }
        _ => {}
    }
    if plan.requires_approval && !request.approval_granted {
        return Err(RuntimeActionError::ApprovalRequired {
            reason: format!("{} (path: {})", plan.reason, path_preview),
        });
    }

    let target = &plan.normalized_path;
    let mut backup_path = None;

    if target.exists() && plan.create_backup {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let stem = target
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "backup".into());
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let backup = parent.join(format!("{stem}.bak.{suffix}"));
        fs::copy(target, &backup)?;
        backup_path = Some(backup);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, &request.content)?;

    Ok(FileWriteResult {
        plan,
        bytes_written: request.content.len(),
        backup_path,
    })
}
