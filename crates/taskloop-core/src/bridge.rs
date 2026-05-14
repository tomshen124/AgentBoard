//! Bridge API for external clients (Electron main process).
//! Exposes TaskLoop as a clean policy/state/memory sidecar.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::context::PromptContextBundle;
use crate::contracts::WorkspaceLayout;
use crate::execution::{
    ExecRequest, ExecResult, FileWriteRequest, FileWriteResult, RuntimeActionError,
};
use crate::memory::{MemoryKind, MemoryRecord};
use crate::model::{Session, Task, TaskHost, TaskKind, TaskPriority};
use crate::policy::{ExecPlan, FileWritePlan, WorkspaceExecutionPolicy};
use crate::runtime::TaskLoopRuntime;
use crate::state::TaskProjection;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub file: String,
    pub line: usize,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub root: PathBuf,
    pub contracts_loaded: bool,
    pub policy: WorkspaceExecutionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCreationRequest {
    pub session_id: String,
    pub title: String,
    pub prompt: String,
}

pub struct TaskLoopBridge {
    runtime: TaskLoopRuntime,
    workspace_root: PathBuf,
}

impl TaskLoopBridge {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let root = workspace_root.into();
        let mut runtime = TaskLoopRuntime::new();
        let layout = WorkspaceLayout::new(&root);
        let _ = runtime.load_workspace_contracts(&layout);
        Self { runtime, workspace_root: root }
    }

    // ── Workspace ──

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            root: self.workspace_root.clone(),
            contracts_loaded: self.runtime.workspace_contracts().is_some(),
            policy: self.runtime.execution_policy(),
        }
    }

    // ── Sessions ──

    pub fn ensure_session(&mut self, id: impl Into<String>) -> Session {
        let id = id.into();
        let session = Session {
            id,
            title: String::new(),
            workspace_root: self.workspace_root.to_string_lossy().to_string(),
        };
        self.runtime.register_session(session.clone());
        session
    }

    // ── Tasks ──

    pub fn create_task(&mut self, req: TaskCreationRequest) -> Result<Task, RuntimeActionError> {
        let task = Task {
            id: format!("task-{}", now_ms()),
            session_id: req.session_id,
            work_item_id: None,
            title: req.title,
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: now_ms(),
        };
        self.runtime.register_task(task.clone());
        let _ = self.runtime.enqueue_task(&task.id, now_ms(), false);
        Ok(task)
    }

    pub fn task_projection(&self, task_id: &str) -> Option<&TaskProjection> {
        self.runtime.task_state(task_id)
    }

    // ── Policy checks ──

    pub fn evaluate_exec(&self, command: &str) -> ExecPlan {
        self.runtime.plan_exec(command)
    }

    pub fn evaluate_file_write(&self, path: &Path, destructive: bool) -> FileWritePlan {
        self.runtime.plan_file_write(path, destructive)
    }

    // ── Read-only tools ──

    pub fn read_file(
        &self,
        relative_path: &Path,
        offset: usize,
        limit: usize,
    ) -> Result<String, RuntimeActionError> {
        let full = self.resolve_workspace_path(relative_path)?;
        if !full.is_file() {
            return Err(RuntimeActionError::Io(format!("not a file: {}", full.display())));
        }
        let raw = std::fs::read_to_string(&full)
            .map_err(|e| RuntimeActionError::Io(format!("read {}: {e}", full.display())))?;
        let lines: Vec<&str> = raw.lines().collect();
        let start = offset.min(lines.len());
        let end = (start.saturating_add(limit)).min(lines.len());
        Ok(lines[start..end].join("\n"))
    }

    pub fn list_dir(&self, relative_path: &Path) -> Result<Vec<DirEntryInfo>, RuntimeActionError> {
        let full = self.resolve_workspace_path(relative_path)?;
        if !full.is_dir() {
            return Err(RuntimeActionError::Io(format!("not a directory: {}", full.display())));
        }
        let mut entries = Vec::new();
        for entry in
            std::fs::read_dir(&full).map_err(|e| RuntimeActionError::Io(format!("read_dir: {e}")))?
        {
            let entry = entry.map_err(|e| RuntimeActionError::Io(format!("dir entry: {e}")))?;
            let meta = entry.metadata().ok();
            entries.push(DirEntryInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    pub fn search_repo(
        &self,
        query: &str,
        relative_path: &Path,
        max_results: usize,
    ) -> Result<Vec<SearchMatch>, RuntimeActionError> {
        let full = self.resolve_workspace_path(relative_path)?;
        let mut results = Vec::new();
        self.search_dir(&full, query, max_results, &mut results)?;
        results.truncate(max_results);
        Ok(results)
    }

    fn search_dir(
        &self,
        dir: &Path,
        query: &str,
        max: usize,
        results: &mut Vec<SearchMatch>,
    ) -> Result<(), RuntimeActionError> {
        if !dir.is_dir() || results.len() >= max {
            return Ok(());
        }
        let skip = ["node_modules", ".git", "target", ".venv", "out", "dist", ".next", "__pycache__", ".agentboard"];
        for entry in
            std::fs::read_dir(dir).map_err(|e| RuntimeActionError::Io(format!("search dir: {e}")))?
        {
            if results.len() >= max {
                break;
            }
            let entry = entry.map_err(|e| RuntimeActionError::Io(format!("entry: {e}")))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || skip.contains(&name.as_str()) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                self.search_dir(&path, query, max, results)?;
            } else {
                if let Ok(meta) = path.metadata() {
                    if meta.len() > 1_000_000 {
                        continue;
                    }
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (i, line) in content.lines().enumerate() {
                        if results.len() >= max {
                            break;
                        }
                        if line.contains(query) {
                            results.push(SearchMatch {
                                file: path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(&path)
                                    .to_string_lossy()
                                    .to_string(),
                                line: i + 1,
                                content: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // ── Exec / File Write ──

    pub fn execute_command(&mut self, req: ExecRequest) -> Result<ExecResult, RuntimeActionError> {
        self.runtime.execute_exec(req)
    }

    pub fn execute_file_write(&mut self, req: FileWriteRequest) -> Result<FileWriteResult, RuntimeActionError> {
        self.runtime.write_file(req)
    }

    // ── Memory ──

    pub fn remember(
        &mut self,
        kind: MemoryKind,
        scope: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) {
        let record = MemoryRecord {
            id: format!("mem-{}", now_ms()),
            kind,
            scope: scope.into(),
            subject_ref: None,
            content: content.into(),
            source: "agent".into(),
            confidence: 80,
            updated_at_ms: now_ms(),
            supersedes: None,
            tags,
        };
        self.runtime.remember(record);
    }

    pub fn recall(&self, scope: Option<&str>, kind: Option<MemoryKind>) -> Vec<&MemoryRecord> {
        self.runtime
            .memory_store()
            .active()
            .into_iter()
            .filter(|r| {
                if let Some(s) = scope {
                    if r.scope != s {
                        return false;
                    }
                }
                if let Some(ref k) = kind {
                    if r.kind != *k {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    // ── Context ──

    pub fn assemble_context(&self, task_id: Option<&str>) -> Result<PromptContextBundle, RuntimeActionError> {
        self.runtime.assemble_prompt_context(task_id, now_ms())
    }

    // ── Persistence ──

    pub fn save(&self) -> Result<(), RuntimeActionError> {
        let store_dir = self.workspace_root.join(".agentboard").join("taskloop");
        std::fs::create_dir_all(&store_dir).map_err(|e| {
            RuntimeActionError::Io(format!("create taskloop dir: {e}"))
        })?;
        let store = crate::persistence::RuntimeStateStore::new(&store_dir);
        self.runtime.save_state(&store).map_err(|e| {
            RuntimeActionError::Io(format!("save state: {e}"))
        })
    }

    pub fn load(&mut self) -> Result<bool, RuntimeActionError> {
        let store_dir = self.workspace_root.join(".agentboard").join("taskloop");
        if !store_dir.exists() {
            return Ok(false);
        }
        let store = crate::persistence::RuntimeStateStore::new(&store_dir);
        match TaskLoopRuntime::load_state(&store) {
            Ok(loaded) => {
                self.runtime = loaded;
                Ok(true)
            }
            Err(e) => {
                eprintln!("[TaskLoop] Failed to load persisted state: {e}");
                Ok(false)
            }
        }
    }

    // ── Approval ──

    pub fn approve_exec(&mut self, task_id: &str, checkpoint: &str) -> Result<(), RuntimeActionError> {
        self.runtime.resume_task_from_checkpoint(task_id, checkpoint, now_ms(), false)
    }

    pub fn reject_exec(&mut self, task_id: &str, checkpoint: &str, reason: &str) -> Result<(), RuntimeActionError> {
        self.runtime.reject_task_checkpoint(task_id, checkpoint, reason, now_ms())
    }

    // ── Internals ──

    fn resolve_workspace_path(&self, relative: &Path) -> Result<PathBuf, RuntimeActionError> {
        let candidate = if relative.is_absolute() {
            relative.to_path_buf()
        } else {
            self.workspace_root.join(relative)
        };
        let canonical = std::fs::canonicalize(&candidate).map_err(|e| {
            RuntimeActionError::Io(format!("resolve {}: {e}", candidate.display()))
        })?;
        if !canonical.starts_with(&self.workspace_root) {
            return Err(RuntimeActionError::InvalidWorkingDirectory {
                reason: format!(
                    "path {} is outside workspace {}",
                    canonical.display(),
                    self.workspace_root.display()
                ),
            });
        }
        Ok(canonical)
    }
}
