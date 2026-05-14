use crate::model::TaskHost;
use crate::policy::ApprovalRiskLevel;
use crate::state::{TaskWaitKind, TaskWorkflowMode};
use crate::model::SubagentStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventVisibility {
    Internal,
    Ui,
    Audit,
    ExternalSync,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventSource {
    Runtime,
    Connector(String),
    Tool(String),
    Model(String),
    User,
    Scheduler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventObjectRef {
    pub object_type: String,
    pub object_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    TaskCreated {
        initial_phase: Option<String>,
        host: TaskHost,
    },
    TaskStarted,
    TaskWorkflowModeChanged {
        mode: TaskWorkflowMode,
        reason: String,
    },
    TaskProgress {
        phase: Option<String>,
        current_step: Option<String>,
        total_steps: Option<u32>,
        progress_text: String,
    },
    TaskHeartbeat {
        message: Option<String>,
    },
    TaskWaiting {
        kind: TaskWaitKind,
        reason: String,
        resume_checkpoint: Option<String>,
    },
    TaskBlocked {
        reason: String,
    },
    TaskResumed,
    TaskCompleted {
        summary: Option<String>,
    },
    TaskFailed {
        error: String,
    },
    TaskCancelled,
    ToolCalled {
        name: String,
    },
    ToolFinished {
        name: String,
    },
    SubagentSpawned {
        subagent_id: String,
        role: String,
        background: bool,
    },
    SubagentUpdated {
        subagent_id: String,
        status: SubagentStatus,
        detail: Option<String>,
    },
    PolicyAllowed {
        action: String,
        detail: String,
        risk_level: ApprovalRiskLevel,
    },
    PolicyDenied {
        action: String,
        reason: String,
        risk_level: ApprovalRiskLevel,
    },
    PolicyApprovalRequired {
        action: String,
        reason: String,
        risk_level: ApprovalRiskLevel,
    },
    PolicyApproved {
        action: String,
        checkpoint: String,
        risk_level: ApprovalRiskLevel,
        decision_source: String,
        resolved_by: String,
    },
    PolicyRejected {
        action: String,
        checkpoint: String,
        reason: String,
        risk_level: ApprovalRiskLevel,
        decision_source: String,
        resolved_by: String,
    },
    ExternalReceived {
        connector_id: String,
    },
    ExternalSynced {
        connector_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEnvelope {
    pub event_id: String,
    pub object: EventObjectRef,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub source: EventSource,
    pub timestamp_ms: u64,
    pub sequence: u64,
    pub visibility: EventVisibility,
    pub event: DomainEvent,
}

impl EventEnvelope {
    pub fn for_task(
        event_id: impl Into<String>,
        task_id: impl Into<String>,
        session_id: impl Into<String>,
        sequence: u64,
        timestamp_ms: u64,
        source: EventSource,
        visibility: EventVisibility,
        event: DomainEvent,
    ) -> Self {
        let task_id = task_id.into();
        Self {
            event_id: event_id.into(),
            object: EventObjectRef {
                object_type: "task".to_string(),
                object_id: task_id.clone(),
            },
            task_id: Some(task_id),
            session_id: Some(session_id.into()),
            source,
            timestamp_ms,
            sequence,
            visibility,
            event,
        }
    }
}
