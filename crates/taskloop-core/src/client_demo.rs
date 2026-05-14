use crate::{
    client_flow::{ClientFlowAdapter, ClientFlowState},
    client_shell::ClientShellState,
    client_view::{build_client_shell_view_model, ClientShellViewModel},
    execution::RuntimeActionError,
    runtime::TaskLoopRuntime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDemoState {
    pub flow: ClientFlowState,
    pub shell: ClientShellState,
    pub view: ClientShellViewModel,
}

pub struct ClientDemoAdapter<'a> {
    flow: ClientFlowAdapter<'a>,
}

impl<'a> ClientDemoAdapter<'a> {
    pub fn new(runtime: &'a mut TaskLoopRuntime) -> Self {
        Self {
            flow: ClientFlowAdapter::new(runtime),
        }
    }

    pub fn bootstrap(&mut self, generated_at_ms: u64) -> ClientDemoState {
        let flow = self.flow.bootstrap(generated_at_ms);
        let shell = flow.shell_state.clone();
        let view = build_client_shell_view_model(&flow);
        ClientDemoState { flow, shell, view }
    }

    pub fn refresh(
        &mut self,
        state: &ClientDemoState,
        generated_at_ms: u64,
    ) -> ClientDemoState {
        let flow = self.flow.refresh(&state.flow, generated_at_ms);
        let shell = flow.shell_state.clone();
        let view = build_client_shell_view_model(&flow);
        ClientDemoState { flow, shell, view }
    }

    pub fn dispatch(
        &mut self,
        state: &ClientDemoState,
        action_key: &str,
        reason: Option<String>,
        timestamp_ms: u64,
    ) -> Result<ClientDemoState, RuntimeActionError> {
        let flow =
            self.flow
                .dispatch_selected_action(&state.flow, action_key, reason, timestamp_ms)?;
        let shell = flow.shell_state.clone();
        let view = build_client_shell_view_model(&flow);
        Ok(ClientDemoState { flow, shell, view })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client_demo::ClientDemoAdapter,
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
    };

    #[test]
    fn client_demo_adapter_bootstraps_view_state() {
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

        let mut demo = ClientDemoAdapter::new(&mut runtime);
        let state = demo.bootstrap(2_000);
        assert_eq!(state.view.header_title, "Task Center");
        assert_eq!(state.shell.snapshot.task_center.summary.total_tasks, 1);
    }
}
