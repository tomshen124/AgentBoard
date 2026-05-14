use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub workspace_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    Internal,
    ExternalHost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskHost {
    Internal,
    External {
        system: String,
        object_type: String,
        object_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub session_id: String,
    pub work_item_id: Option<String>,
    pub title: String,
    pub kind: TaskKind,
    pub host: TaskHost,
    pub priority: TaskPriority,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub source_system: String,
    pub summary: String,
    pub channel_object_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelObject {
    pub id: String,
    pub source_system: String,
    pub object_type: String,
    pub external_id: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub task_id: String,
    pub name: String,
    pub path: String,
    pub expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentStatus {
    Queued,
    Running,
    Waiting,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subagent {
    pub id: String,
    pub parent_task_id: String,
    pub role: String,
    pub status: SubagentStatus,
    pub detail: Option<String>,
    pub background: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
