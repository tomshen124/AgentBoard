use crate::{
    client_api::{ClientActionRequest, ClientApiSnapshot},
    client_runtime_api::{ClientActionExecution, ClientRuntimeApi},
    execution::RuntimeActionError,
    runtime::TaskLoopRuntime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientNotificationLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientNotification {
    pub id: String,
    pub level: ClientNotificationLevel,
    pub message: String,
    pub action_key: Option<String>,
    pub notification_key: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientShellState {
    pub snapshot: ClientApiSnapshot,
    pub notifications: Vec<ClientNotification>,
}

pub struct ClientShellAdapter<'a> {
    api: ClientRuntimeApi<'a>,
    notifications: Vec<ClientNotification>,
    next_notification: u64,
}

impl<'a> ClientShellAdapter<'a> {
    pub fn new(runtime: &'a mut TaskLoopRuntime) -> Self {
        Self {
            api: ClientRuntimeApi::new(runtime),
            notifications: Vec::new(),
            next_notification: 0,
        }
    }

    pub fn bootstrap(&mut self, generated_at_ms: u64) -> ClientShellState {
        let snapshot = self.api.snapshot(generated_at_ms);
        self.rebuild_snapshot_notifications(&snapshot, generated_at_ms);
        ClientShellState {
            snapshot,
            notifications: self.notifications.clone(),
        }
    }

    pub fn refresh(&mut self, generated_at_ms: u64) -> ClientShellState {
        self.bootstrap(generated_at_ms)
    }

    pub fn dispatch(
        &mut self,
        request: ClientActionRequest,
        refreshed_at_ms: u64,
    ) -> Result<ClientShellState, RuntimeActionError> {
        let action_key = request.action_key.clone();
        let execution = self.api.execute_and_refresh(request, refreshed_at_ms)?;
        let notification_id = self.next_notification_id();
        let notification =
            notification_from_execution(notification_id, &execution, refreshed_at_ms);
        self.push_notification(notification);
        let mut state = ClientShellState {
            snapshot: execution.snapshot,
            notifications: self.notifications.clone(),
        };
        if execution.result.failed.is_empty() {
            state
                .notifications
                .last_mut()
                .map(|note| note.action_key = Some(action_key));
        }
        Ok(state)
    }

    fn next_notification_id(&mut self) -> String {
        self.next_notification += 1;
        format!("client-note-{}", self.next_notification)
    }

    fn push_notification(&mut self, notification: ClientNotification) {
        self.notifications.push(notification);
        if self.notifications.len() > 20 {
            let drain = self.notifications.len() - 20;
            self.notifications.drain(0..drain);
        }
    }

    fn rebuild_snapshot_notifications(
        &mut self,
        snapshot: &ClientApiSnapshot,
        timestamp_ms: u64,
    ) {
        self.notifications
            .retain(|note| note.notification_key.is_none());

        if snapshot.approval_center.summary.pending_high_risk > 0 {
            let id = self.next_notification_id();
            self.push_notification(ClientNotification {
                id,
                level: ClientNotificationLevel::Warn,
                message: format!(
                    "{} high-risk approval(s) pending",
                    snapshot.approval_center.summary.pending_high_risk
                ),
                action_key: Some("approve_selected".into()),
                notification_key: Some("approval:high_risk_pending".into()),
                timestamp_ms,
            });
        }

        if snapshot.task_center.summary.waiting_tasks > 0 {
            let id = self.next_notification_id();
            self.push_notification(ClientNotification {
                id,
                level: ClientNotificationLevel::Info,
                message: format!(
                    "{} task(s) currently waiting",
                    snapshot.task_center.summary.waiting_tasks
                ),
                action_key: Some("open_waiting_tasks".into()),
                notification_key: Some("task:waiting".into()),
                timestamp_ms,
            });
        }

        if snapshot.connector_center.summary.attention_hosts > 0 {
            let id = self.next_notification_id();
            self.push_notification(ClientNotification {
                id,
                level: ClientNotificationLevel::Info,
                message: format!(
                    "{} connector host(s) need attention",
                    snapshot.connector_center.summary.attention_hosts
                ),
                action_key: Some("open_attention_inbox".into()),
                notification_key: Some("connector:attention_hosts".into()),
                timestamp_ms,
            });
        }
    }
}

fn notification_from_execution(
    id: String,
    execution: &ClientActionExecution,
    timestamp_ms: u64,
) -> ClientNotification {
    let failed = execution.result.failed.len() as u32;
    let succeeded = execution.result.succeeded.len() as u32;
    let (level, message) = if failed == 0 {
        (
            ClientNotificationLevel::Info,
            format!(
                "{} completed for {} item(s)",
                execution.result.action_key, succeeded
            ),
        )
    } else if succeeded > 0 {
        (
            ClientNotificationLevel::Warn,
            format!(
                "{} partially completed: {} succeeded, {} failed",
                execution.result.action_key, succeeded, failed
            ),
        )
    } else {
        (
            ClientNotificationLevel::Error,
            format!("{} failed for {} item(s)", execution.result.action_key, failed),
        )
    };

    ClientNotification {
        id,
        level,
        message,
        action_key: Some(execution.result.action_key.clone()),
        notification_key: None,
        timestamp_ms,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client_api::{ClientActionRequest, ClientCenterKind},
        client_shell::{ClientNotificationLevel, ClientShellAdapter},
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
    };

    #[test]
    fn client_shell_adapter_dispatches_and_collects_notifications() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Task 1".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        runtime.enqueue_task("task-1", 1_500, false).unwrap();

        let mut shell = ClientShellAdapter::new(&mut runtime);
        let initial = shell.bootstrap(1_900);
        assert_eq!(initial.snapshot.task_center.summary.total_tasks, 1);
        assert!(initial.notifications.is_empty());

        let updated = shell
            .dispatch(
                ClientActionRequest {
                    center: ClientCenterKind::Task,
                    action_key: "cancel_selected".into(),
                    item_keys: vec!["task-1".into()],
                    reason: Some("user cancelled".into()),
                    timestamp_ms: 2_000,
                    background: false,
                },
                2_100,
            )
            .unwrap();

        assert_eq!(updated.notifications.len(), 1);
        assert_eq!(updated.notifications[0].level, ClientNotificationLevel::Info);
        assert!(updated.notifications[0].message.contains("cancel_selected"));
    }

    #[test]
    fn client_shell_adapter_generates_snapshot_notifications() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Task 1".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            crate::DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::ApprovalRiskLevel::High,
            },
            1_400,
            crate::EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            crate::DomainEvent::TaskWaiting {
                kind: crate::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            crate::EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let mut shell = ClientShellAdapter::new(&mut runtime);
        let state = shell.refresh(2_000);
        assert!(state
            .notifications
            .iter()
            .any(|note| note.notification_key.as_deref() == Some("approval:high_risk_pending")));
        assert!(state
            .notifications
            .iter()
            .any(|note| note.notification_key.as_deref() == Some("task:waiting")));
    }
}
