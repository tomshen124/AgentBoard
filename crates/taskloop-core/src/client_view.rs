use crate::{
    client_flow::{active_center_actions, ClientCenterRoute, ClientFlowState},
    client_shell::{ClientNotification, ClientNotificationLevel},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientShellViewModel {
    pub active_center: String,
    pub header_title: String,
    pub header_subtitle: String,
    pub badges: Vec<ClientShellBadge>,
    pub notifications: Vec<ClientNotification>,
    pub primary_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientShellBadge {
    pub key: String,
    pub label: String,
    pub tone: String,
    pub count: u32,
}

pub fn build_client_shell_view_model(state: &ClientFlowState) -> ClientShellViewModel {
    let snapshot = &state.shell_state.snapshot;
    let (header_title, header_subtitle) = match state.active_center {
        ClientCenterRoute::Task => (
            "Task Center".into(),
            format!(
                "{} task(s), {} waiting",
                snapshot.task_center.summary.total_tasks,
                snapshot.task_center.summary.waiting_tasks
            ),
        ),
        ClientCenterRoute::Approval => (
            "Approval Center".into(),
            format!(
                "{} pending, {} high-risk",
                snapshot.approval_center.summary.pending_total,
                snapshot.approval_center.summary.pending_high_risk
            ),
        ),
        ClientCenterRoute::Connector => (
            "Connector Center".into(),
            format!(
                "{} host(s), {} syncable",
                snapshot.connector_center.summary.total_hosts,
                snapshot.connector_center.summary.syncable_hosts
            ),
        ),
    };

    let badges = vec![
        ClientShellBadge {
            key: "tasks".into(),
            label: "Tasks".into(),
            tone: "neutral".into(),
            count: snapshot.task_center.summary.total_tasks,
        },
        ClientShellBadge {
            key: "approvals".into(),
            label: "Pending Approvals".into(),
            tone: if snapshot.approval_center.summary.pending_high_risk > 0 {
                "warn".into()
            } else {
                "neutral".into()
            },
            count: snapshot.approval_center.summary.pending_total,
        },
        ClientShellBadge {
            key: "hosts".into(),
            label: "Connector Hosts".into(),
            tone: if snapshot.connector_center.summary.attention_hosts > 0 {
                "warn".into()
            } else {
                "neutral".into()
            },
            count: snapshot.connector_center.summary.total_hosts,
        },
    ];

    let primary_actions = active_center_actions(state)
        .into_iter()
        .filter(|action| action.enabled)
        .take(3)
        .map(|action| action.action_key)
        .collect();

    ClientShellViewModel {
        active_center: center_label(&state.active_center),
        header_title,
        header_subtitle,
        badges,
        notifications: normalized_notifications(&state.shell_state.notifications),
        primary_actions,
    }
}

fn center_label(center: &ClientCenterRoute) -> String {
    match center {
        ClientCenterRoute::Task => "task".into(),
        ClientCenterRoute::Approval => "approval".into(),
        ClientCenterRoute::Connector => "connector".into(),
    }
}

fn normalized_notifications(
    notifications: &[ClientNotification],
) -> Vec<ClientNotification> {
    let mut items = notifications.to_vec();
    items.sort_by(|left, right| {
        notification_rank(&left.level)
            .cmp(&notification_rank(&right.level))
            .then_with(|| right.timestamp_ms.cmp(&left.timestamp_ms))
    });
    items
}

fn notification_rank(level: &ClientNotificationLevel) -> u8 {
    match level {
        ClientNotificationLevel::Error => 0,
        ClientNotificationLevel::Warn => 1,
        ClientNotificationLevel::Info => 2,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client_flow::{ClientFlowAdapter, ClientCenterRoute},
        client_view::build_client_shell_view_model,
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
        DomainEvent, EventVisibility,
    };

    #[test]
    fn client_view_prefers_approval_header_when_approval_center_active() {
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
        let approval = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval);
        let waiting = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting);

        let mut flow = ClientFlowAdapter::new(&mut runtime);
        let state = flow.bootstrap(2_000);
        assert_eq!(state.active_center, ClientCenterRoute::Approval);

        let view = build_client_shell_view_model(&state);
        assert_eq!(view.active_center, "approval");
        assert_eq!(view.header_title, "Approval Center");
        assert!(view.header_subtitle.contains("high-risk"));
        assert!(view.primary_actions.iter().any(|item| item == "approve_selected"));
    }
}
