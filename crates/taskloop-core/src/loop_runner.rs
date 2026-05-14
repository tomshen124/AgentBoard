use crate::{
    execution::{ExecRequest, FileWriteRequest, RuntimeActionError},
    memory::MemoryRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAction {
    Progress {
        phase: Option<String>,
        current_step: Option<String>,
        total_steps: Option<u32>,
        message: String,
    },
    Heartbeat {
        message: Option<String>,
    },
    Exec {
        request: ExecRequest,
    },
    WriteFile {
        request: FileWriteRequest,
    },
    Remember {
        record: MemoryRecord,
    },
    ScheduleWakeup {
        wake_at_ms: u64,
        reason: String,
    },
    Complete {
        summary: Option<String>,
    },
    Fail {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskScript {
    pub actions: Vec<TaskAction>,
    pub auto_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunReport {
    pub task_id: String,
    pub action_count: usize,
    pub completed: bool,
    pub waiting_for_wakeup: bool,
    pub waiting_for_approval: bool,
    pub resume_checkpoint: Option<String>,
    pub failed: bool,
    pub replanned_actions: usize,
}

impl TaskScript {
    pub fn new(actions: Vec<TaskAction>) -> Self {
        Self {
            actions,
            auto_complete: true,
        }
    }
}

pub(crate) fn map_action_error(error: RuntimeActionError) -> RuntimeActionError {
    error
}
