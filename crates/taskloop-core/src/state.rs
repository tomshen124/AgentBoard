use serde::{Deserialize, Serialize};
use crate::event::{DomainEvent, EventEnvelope};
use crate::model::{SubagentStatus, Task, TaskHost};
use crate::policy::ApprovalRiskLevel;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskWaitKind {
    User,
    Approval,
    Tool,
    Schedule,
    Subtask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskWorkflowMode {
    Simple,
    Complex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskLifecycleState {
    Queued,
    Running,
    Waiting,
    Paused,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskHealthState {
    Healthy,
    Degraded,
    Stalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProjection {
    pub task_id: String,
    pub session_id: String,
    pub title: String,
    pub lifecycle: TaskLifecycleState,
    pub workflow_mode: TaskWorkflowMode,
    pub health: TaskHealthState,
    pub host: TaskHost,
    pub phase: Option<String>,
    pub current_step: Option<String>,
    pub total_steps: Option<u32>,
    pub progress_text: Option<String>,
    pub waiting_on: Option<TaskWaitKind>,
    pub blocked_reason: Option<String>,
    pub approval_action: Option<String>,
    pub approval_risk: Option<ApprovalRiskLevel>,
    pub approval_requested_at_ms: Option<u64>,
    pub resume_checkpoint: Option<String>,
    pub last_heartbeat_at_ms: Option<u64>,
    pub expected_next_update_at_ms: Option<u64>,
    pub active_subagents: u32,
}

impl TaskProjection {
    pub fn from_task(task: &Task) -> Self {
        Self {
            task_id: task.id.clone(),
            session_id: task.session_id.clone(),
            title: task.title.clone(),
            lifecycle: TaskLifecycleState::Queued,
            workflow_mode: TaskWorkflowMode::Simple,
            health: TaskHealthState::Healthy,
            host: task.host.clone(),
            phase: None,
            current_step: None,
            total_steps: None,
            progress_text: None,
            waiting_on: None,
            blocked_reason: None,
            approval_action: None,
            approval_risk: None,
            approval_requested_at_ms: None,
            resume_checkpoint: None,
            last_heartbeat_at_ms: Some(task.created_at_ms),
            expected_next_update_at_ms: None,
            active_subagents: 0,
        }
    }

    pub fn apply_event(&mut self, envelope: &EventEnvelope) {
        self.last_heartbeat_at_ms = Some(envelope.timestamp_ms);
        self.health = TaskHealthState::Healthy;

        match &envelope.event {
            DomainEvent::TaskCreated { initial_phase, host } => {
                self.phase = initial_phase.clone();
                self.host = host.clone();
            }
            DomainEvent::TaskStarted => {
                self.lifecycle = TaskLifecycleState::Running;
                self.waiting_on = None;
                self.blocked_reason = None;
                self.approval_action = None;
                self.approval_risk = None;
                self.approval_requested_at_ms = None;
                self.resume_checkpoint = None;
            }
            DomainEvent::TaskWorkflowModeChanged { mode, reason } => {
                self.workflow_mode = mode.clone();
                self.progress_text = Some(format!("workflow mode switched: {reason}"));
            }
            DomainEvent::TaskProgress {
                phase,
                current_step,
                total_steps,
                progress_text,
            } => {
                self.lifecycle = TaskLifecycleState::Running;
                self.phase = phase.clone().or_else(|| self.phase.clone());
                self.current_step = current_step.clone();
                self.total_steps = *total_steps;
                self.progress_text = Some(progress_text.clone());
                self.waiting_on = None;
                self.blocked_reason = None;
                self.approval_action = None;
                self.approval_risk = None;
                self.approval_requested_at_ms = None;
                self.resume_checkpoint = None;
            }
            DomainEvent::TaskHeartbeat { message } => {
                self.lifecycle = TaskLifecycleState::Running;
                if let Some(message) = message {
                    self.progress_text = Some(message.clone());
                }
            }
            DomainEvent::TaskWaiting {
                kind,
                reason,
                resume_checkpoint,
            } => {
                self.lifecycle = TaskLifecycleState::Waiting;
                self.waiting_on = Some(kind.clone());
                self.blocked_reason = Some(reason.clone());
                if matches!(kind, TaskWaitKind::Approval) {
                    self.approval_requested_at_ms = Some(envelope.timestamp_ms);
                }
                self.resume_checkpoint = resume_checkpoint.clone();
            }
            DomainEvent::TaskBlocked { reason } => {
                self.lifecycle = TaskLifecycleState::Waiting;
                self.blocked_reason = Some(reason.clone());
            }
            DomainEvent::TaskResumed => {
                self.lifecycle = TaskLifecycleState::Running;
                self.waiting_on = None;
                self.blocked_reason = None;
                self.approval_action = None;
                self.approval_risk = None;
                self.approval_requested_at_ms = None;
                self.resume_checkpoint = None;
            }
            DomainEvent::TaskCompleted { summary } => {
                self.lifecycle = TaskLifecycleState::Done;
                self.progress_text = summary.clone();
                self.waiting_on = None;
                self.blocked_reason = None;
                self.approval_action = None;
                self.approval_risk = None;
                self.approval_requested_at_ms = None;
                self.resume_checkpoint = None;
            }
            DomainEvent::TaskFailed { error } => {
                self.lifecycle = TaskLifecycleState::Failed;
                self.blocked_reason = Some(error.clone());
                self.waiting_on = None;
                self.resume_checkpoint = None;
                self.approval_requested_at_ms = None;
            }
            DomainEvent::TaskCancelled => {
                self.lifecycle = TaskLifecycleState::Cancelled;
                self.waiting_on = None;
                self.approval_action = None;
                self.approval_risk = None;
                self.approval_requested_at_ms = None;
                self.resume_checkpoint = None;
            }
            DomainEvent::ToolCalled { name } => {
                self.progress_text = Some(format!("tool running: {name}"));
            }
            DomainEvent::ToolFinished { name } => {
                self.progress_text = Some(format!("tool finished: {name}"));
            }
            DomainEvent::SubagentSpawned {
                subagent_id: _,
                role,
                background: _,
            } => {
                self.active_subagents = self.active_subagents.saturating_add(1);
                self.waiting_on = Some(TaskWaitKind::Subtask);
                self.progress_text = Some(format!("subagent spawned: {role}"));
            }
            DomainEvent::SubagentUpdated {
                subagent_id: _,
                status,
                detail,
            } => {
                match status {
                    SubagentStatus::Queued | SubagentStatus::Running | SubagentStatus::Waiting => {
                        self.waiting_on = Some(TaskWaitKind::Subtask);
                    }
                    SubagentStatus::Done | SubagentStatus::Failed => {
                        self.active_subagents = self.active_subagents.saturating_sub(1);
                        if self.active_subagents == 0 {
                            self.waiting_on = None;
                        }
                    }
                }
                if let Some(detail) = detail {
                    self.progress_text = Some(format!("subagent update: {detail}"));
                }
            }
            DomainEvent::PolicyAllowed { action, detail, risk_level } => {
                self.progress_text = Some(format!("policy allowed {action}: {detail}"));
                self.approval_action = Some(action.clone());
                self.approval_risk = Some(risk_level.clone());
            }
            DomainEvent::PolicyDenied { action, reason, risk_level } => {
                self.progress_text = Some(format!("policy denied {action}: {reason}"));
                self.approval_action = Some(action.clone());
                self.approval_risk = Some(risk_level.clone());
            }
            DomainEvent::PolicyApprovalRequired { action, reason, risk_level } => {
                self.progress_text = Some(format!("approval required for {action}: {reason}"));
                self.approval_action = Some(action.clone());
                self.approval_risk = Some(risk_level.clone());
            }
            DomainEvent::PolicyApproved {
                action,
                checkpoint: _,
                risk_level,
                decision_source: _,
                resolved_by: _,
            } => {
                self.progress_text = Some(format!("approval approved for {action}"));
                self.approval_action = Some(action.clone());
                self.approval_risk = Some(risk_level.clone());
            }
            DomainEvent::PolicyRejected {
                action,
                checkpoint: _,
                reason,
                risk_level,
                decision_source: _,
                resolved_by: _,
            } => {
                self.progress_text = Some(format!("approval rejected for {action}: {reason}"));
                self.approval_action = Some(action.clone());
                self.approval_risk = Some(risk_level.clone());
            }
            DomainEvent::ExternalReceived { connector_id } => {
                self.progress_text = Some(format!("external event received via {connector_id}"));
            }
            DomainEvent::ExternalSynced { connector_id } => {
                self.progress_text = Some(format!("external state synced via {connector_id}"));
            }
        }
    }

    pub fn apply_watchdog(
        &mut self,
        now_ms: u64,
        degrade_after_ms: u64,
        stall_after_ms: u64,
    ) {
        let Some(last_seen) = self.last_heartbeat_at_ms else {
            return;
        };

        let elapsed = now_ms.saturating_sub(last_seen);
        self.expected_next_update_at_ms = Some(last_seen.saturating_add(degrade_after_ms));

        if matches!(
            self.lifecycle,
            TaskLifecycleState::Done | TaskLifecycleState::Failed | TaskLifecycleState::Cancelled
        ) {
            self.health = TaskHealthState::Healthy;
            return;
        }

        self.health = if elapsed >= stall_after_ms {
            TaskHealthState::Stalled
        } else if elapsed >= degrade_after_ms {
            TaskHealthState::Degraded
        } else {
            TaskHealthState::Healthy
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::event::{DomainEvent, EventEnvelope, EventSource, EventVisibility};
    use crate::model::{Task, TaskHost, TaskKind, TaskPriority};

    use super::{TaskHealthState, TaskProjection};

    #[test]
    fn reducer_marks_task_stalled_after_watchdog_threshold() {
        let task = Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Example".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        };
        let mut projection = TaskProjection::from_task(&task);
        let event = EventEnvelope::for_task(
            "event-1",
            "task-1",
            "session-1",
            1,
            2_000,
            EventSource::Runtime,
            EventVisibility::Ui,
            DomainEvent::TaskStarted,
        );

        projection.apply_event(&event);
        projection.apply_watchdog(20_000, 5_000, 10_000);

        assert_eq!(projection.health, TaskHealthState::Stalled);
    }
}
