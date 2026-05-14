use crate::{
    client_api::{ClientActionDescriptor, ClientActionRequest, ClientApiSnapshot, ClientCenterKind},
    client_shell::{ClientShellAdapter, ClientShellState},
    execution::RuntimeActionError,
    runtime::TaskLoopRuntime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCenterRoute {
    Task,
    Approval,
    Connector,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientSelectionState {
    pub task_ids: Vec<String>,
    pub approval_ids: Vec<String>,
    pub host_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientFlowState {
    pub active_center: ClientCenterRoute,
    pub shell_state: ClientShellState,
    pub selections: ClientSelectionState,
}

pub struct ClientFlowAdapter<'a> {
    shell: ClientShellAdapter<'a>,
}

impl<'a> ClientFlowAdapter<'a> {
    pub fn new(runtime: &'a mut TaskLoopRuntime) -> Self {
        Self {
            shell: ClientShellAdapter::new(runtime),
        }
    }

    pub fn bootstrap(&mut self, generated_at_ms: u64) -> ClientFlowState {
        let shell_state = self.shell.bootstrap(generated_at_ms);
        ClientFlowState {
            active_center: recommended_center(&shell_state.snapshot),
            shell_state,
            selections: ClientSelectionState::default(),
        }
    }

    pub fn refresh(
        &mut self,
        state: &ClientFlowState,
        generated_at_ms: u64,
    ) -> ClientFlowState {
        let shell_state = self.shell.refresh(generated_at_ms);
        ClientFlowState {
            active_center: recommended_center(&shell_state.snapshot),
            shell_state,
            selections: state.selections.clone(),
        }
    }

    pub fn switch_center(
        &self,
        state: &ClientFlowState,
        active_center: ClientCenterRoute,
    ) -> ClientFlowState {
        ClientFlowState {
            active_center,
            shell_state: state.shell_state.clone(),
            selections: state.selections.clone(),
        }
    }

    pub fn select_items(
        &self,
        state: &ClientFlowState,
        item_keys: Vec<String>,
    ) -> ClientFlowState {
        let mut selections = state.selections.clone();
        match state.active_center {
            ClientCenterRoute::Task => selections.task_ids = item_keys,
            ClientCenterRoute::Approval => selections.approval_ids = item_keys,
            ClientCenterRoute::Connector => selections.host_keys = item_keys,
        }
        ClientFlowState {
            active_center: state.active_center.clone(),
            shell_state: state.shell_state.clone(),
            selections,
        }
    }

    pub fn dispatch_selected_action(
        &mut self,
        state: &ClientFlowState,
        action_key: &str,
        reason: Option<String>,
        timestamp_ms: u64,
    ) -> Result<ClientFlowState, RuntimeActionError> {
        let request = ClientActionRequest {
            center: center_kind(&state.active_center),
            action_key: action_key.to_string(),
            item_keys: selected_item_keys(&state.active_center, &state.selections),
            reason,
            timestamp_ms,
            background: false,
        };
        let shell_state = self.shell.dispatch(request, timestamp_ms)?;
        Ok(ClientFlowState {
            active_center: recommended_center(&shell_state.snapshot),
            shell_state,
            selections: ClientSelectionState::default(),
        })
    }
}

pub fn recommended_center(snapshot: &ClientApiSnapshot) -> ClientCenterRoute {
    if snapshot.approval_center.summary.pending_high_risk > 0 {
        ClientCenterRoute::Approval
    } else if snapshot.task_center.summary.waiting_tasks > 0 {
        ClientCenterRoute::Task
    } else if snapshot.connector_center.summary.attention_hosts > 0 {
        ClientCenterRoute::Connector
    } else {
        ClientCenterRoute::Task
    }
}

pub fn active_center_actions(state: &ClientFlowState) -> Vec<ClientActionDescriptor> {
    match state.active_center {
        ClientCenterRoute::Task => state.shell_state.snapshot.task_center.available_actions.clone(),
        ClientCenterRoute::Approval => {
            state.shell_state.snapshot.approval_center.available_actions.clone()
        }
        ClientCenterRoute::Connector => {
            state.shell_state.snapshot.connector_center.available_actions.clone()
        }
    }
}

fn center_kind(center: &ClientCenterRoute) -> ClientCenterKind {
    match center {
        ClientCenterRoute::Task => ClientCenterKind::Task,
        ClientCenterRoute::Approval => ClientCenterKind::Approval,
        ClientCenterRoute::Connector => ClientCenterKind::Connector,
    }
}

fn selected_item_keys(
    center: &ClientCenterRoute,
    selections: &ClientSelectionState,
) -> Vec<String> {
    match center {
        ClientCenterRoute::Task => selections.task_ids.clone(),
        ClientCenterRoute::Approval => selections.approval_ids.clone(),
        ClientCenterRoute::Connector => selections.host_keys.clone(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client_flow::{active_center_actions, ClientCenterRoute, ClientFlowAdapter},
        client_api::ClientCenterKind,
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
        DomainEvent, EventVisibility,
    };

    #[test]
    fn client_flow_prefers_approval_center_when_high_risk_pending() {
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

        let mut adapter = ClientFlowAdapter::new(&mut runtime);
        let state = adapter.bootstrap(2_000);
        assert_eq!(state.active_center, ClientCenterRoute::Approval);
        assert!(active_center_actions(&state)
            .iter()
            .any(|action| action.action_key == "approve_selected"));
    }

    #[test]
    fn client_flow_dispatches_selected_action() {
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

        let mut adapter = ClientFlowAdapter::new(&mut runtime);
        let state = adapter.bootstrap(1_900);
        assert_eq!(state.active_center, ClientCenterRoute::Task);
        let state = adapter.select_items(&state, vec!["task-1".into()]);
        let state = adapter
            .dispatch_selected_action(&state, "cancel_selected", Some("user cancelled".into()), 2_000)
            .unwrap();
        assert_eq!(state.active_center, ClientCenterRoute::Task);
        assert!(state
            .shell_state
            .notifications
            .iter()
            .any(|note| note.action_key.as_deref() == Some("cancel_selected")));
    }

    #[test]
    fn center_kind_matches_routes() {
        assert_eq!(
            super::center_kind(&ClientCenterRoute::Task),
            ClientCenterKind::Task
        );
        assert_eq!(
            super::center_kind(&ClientCenterRoute::Approval),
            ClientCenterKind::Approval
        );
        assert_eq!(
            super::center_kind(&ClientCenterRoute::Connector),
            ClientCenterKind::Connector
        );
    }
}
