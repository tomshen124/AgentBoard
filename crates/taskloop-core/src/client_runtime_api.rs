use crate::{
    client_api::{ClientActionRequest, ClientActionResult, ClientApiSnapshot},
    execution::RuntimeActionError,
    runtime::TaskLoopRuntime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientActionExecution {
    pub result: ClientActionResult,
    pub snapshot: ClientApiSnapshot,
}

pub struct ClientRuntimeApi<'a> {
    runtime: &'a mut TaskLoopRuntime,
}

impl<'a> ClientRuntimeApi<'a> {
    pub fn new(runtime: &'a mut TaskLoopRuntime) -> Self {
        Self { runtime }
    }

    pub fn snapshot(&self, generated_at_ms: u64) -> ClientApiSnapshot {
        self.runtime.client_api_snapshot(generated_at_ms)
    }

    pub fn execute(
        &mut self,
        request: ClientActionRequest,
    ) -> Result<ClientActionResult, RuntimeActionError> {
        self.runtime.execute_client_action(request)
    }

    pub fn execute_and_refresh(
        &mut self,
        request: ClientActionRequest,
        refreshed_at_ms: u64,
    ) -> Result<ClientActionExecution, RuntimeActionError> {
        let result = self.runtime.execute_client_action(request)?;
        let snapshot = self.runtime.client_api_snapshot(refreshed_at_ms);
        Ok(ClientActionExecution { result, snapshot })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client_api::{ClientActionRequest, ClientCenterKind},
        client_runtime_api::ClientRuntimeApi,
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
    };

    #[test]
    fn client_runtime_api_executes_and_refreshes() {
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

        let mut api = ClientRuntimeApi::new(&mut runtime);
        let initial = api.snapshot(1_900);
        assert_eq!(initial.task_center.summary.total_tasks, 1);

        let execution = api
            .execute_and_refresh(
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

        assert_eq!(execution.result.action_key, "cancel_selected");
        assert_eq!(execution.result.succeeded, vec!["task-1"]);
        assert_eq!(execution.snapshot.task_center.summary.total_tasks, 1);
    }
}
