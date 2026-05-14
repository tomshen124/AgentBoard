use std::collections::HashMap;

use crate::{
    artifact::{cleanup_expired_artifacts, export_artifact, stage_artifact},
    client_api::{
        build_client_api_snapshot, map_approval_batch_result, map_connector_batch_result,
        map_task_batch_result, ClientActionRequest, ClientActionResult, ClientApiSnapshot,
        ClientCenterKind,
    },
    connector::{ConnectorEnvelope, ConnectorSample},
    context::{assemble_prompt_context, PromptContextBundle},
    contracts::{ContractLoadError, WorkspaceContracts, WorkspaceLayout},
    diagnostics::{build_task_diagnostic_report, TaskDiagnosticReport},
    event::{DomainEvent, EventEnvelope, EventSource, EventVisibility},
    execution::{
        run_exec, run_file_write, ExecRequest, ExecResult, FileWriteRequest, FileWriteResult,
        RuntimeActionError,
    },
    loop_runner::{map_action_error, TaskAction, TaskRunReport, TaskScript},
    memory::{render_memory_markdown, MemoryRecord, MemoryStore},
    model::{Artifact, ChannelObject, Session, Subagent, SubagentStatus, Task, WorkItem},
    policy::{ExecPlan, FileWritePlan, WorkspaceExecutionPolicy},
    persistence::{PersistenceError, RuntimeStateStore},
    presentation::{
        build_approval_surface_entries, build_channel_host_surface_entries,
        build_channel_inbox_entries, build_channel_source_counts, build_client_surface_snapshot,
        build_provider_surface_entries, build_session_surface_entries, build_task_board_entry,
        build_task_board_snapshot, build_wait_bucket_counts, build_work_item_surface_entries,
        approval_id_for, approval_risk_label, build_approval_center_summary,
        build_approval_group_buckets, build_approval_history_entries,
        build_approval_view_presets, apply_approval_history_window,
        channel_host_kind_label, channel_host_runtime_key, connector_sync_mode_label,
        host_capability_labels, host_label_for_task_host, ClientSurfaceSnapshot,
        DEFAULT_APPROVAL_HISTORY_LIMIT, TaskAttentionLane, TaskBoardEntry, TaskBoardSnapshot,
    },
    provider::{default_provider_config_path, ProviderCatalog},
    registry::{SkillRegistry, ToolRegistry},
    runtime_log::{RuntimeLogEntry, RuntimeLogLevel},
    scheduler::{QueuedTask, ScheduledWakeup, TaskQueue},
    state::{TaskProjection, TaskWorkflowMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBatchFailure {
    pub approval_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBatchResult {
    pub requested: u32,
    pub succeeded: Vec<String>,
    pub failed: Vec<ApprovalBatchFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBatchFailure {
    pub task_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBatchResult {
    pub requested: u32,
    pub succeeded: Vec<String>,
    pub failed: Vec<TaskBatchFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorBatchFailure {
    pub host_key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorBatchResult {
    pub requested: u32,
    pub succeeded: Vec<String>,
    pub failed: Vec<ConnectorBatchFailure>,
}

#[derive(Debug, Default)]
pub struct TaskLoopRuntime {
    sessions: HashMap<String, Session>,
    tasks: HashMap<String, Task>,
    artifacts: HashMap<String, Artifact>,
    subagents: HashMap<String, Subagent>,
    channel_objects: HashMap<String, ChannelObject>,
    work_items: HashMap<String, WorkItem>,
    task_state: HashMap<String, TaskProjection>,
    task_attention: HashMap<String, bool>,
    events: Vec<EventEnvelope>,
    logs: Vec<RuntimeLogEntry>,
    workspace_contracts: Option<WorkspaceContracts>,
    workspace_root: Option<std::path::PathBuf>,
    provider_catalog: ProviderCatalog,
    tool_registry: ToolRegistry,
    skill_registry: SkillRegistry,
    scheduler: TaskQueue,
    memory_store: MemoryStore,
    next_sequence: u64,
}

impl TaskLoopRuntime {
    pub fn new() -> Self {
        Self {
            tool_registry: ToolRegistry::with_defaults(),
            skill_registry: SkillRegistry::with_builtin_defaults(),
            ..Self::default()
        }
    }

    pub fn register_session(&mut self, session: Session) {
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn load_workspace_contracts(
        &mut self,
        layout: &WorkspaceLayout,
    ) -> Result<&WorkspaceContracts, ContractLoadError> {
        let loaded = layout.load_contracts()?;
        self.workspace_root = Some(layout.root().to_path_buf());
        self.provider_catalog = ProviderCatalog::load(default_provider_config_path(layout.root()))?;
        self.skill_registry = SkillRegistry::from_workspace_contracts(&loaded);
        self.workspace_contracts = Some(loaded);
        Ok(self.workspace_contracts.as_ref().expect("just inserted"))
    }

    pub fn load_state(store: &RuntimeStateStore) -> Result<Self, PersistenceError> {
        let (
            workspace_root,
            next_sequence,
            sessions,
            tasks,
            artifacts,
            subagents,
            channel_objects,
            work_items,
            events,
            logs,
            queued,
            wakeups,
            memory_records,
        ) = store.load()?;
        let mut runtime = Self {
            workspace_root,
            provider_catalog: ProviderCatalog::default(),
            tool_registry: ToolRegistry::with_defaults(),
            skill_registry: SkillRegistry::with_builtin_defaults(),
            scheduler: TaskQueue::from_parts(queued, wakeups),
            memory_store: MemoryStore::from_records(memory_records),
            sessions,
            tasks,
            artifacts,
            subagents,
            channel_objects,
            work_items,
            logs,
            task_attention: HashMap::new(),
            next_sequence,
            ..Self::default()
        };
        runtime.rebuild_projections();
        runtime.rebuild_task_attention();
        for event in events {
            runtime.append_event(event);
        }
        Ok(runtime)
    }

    pub fn save_state(&self, store: &RuntimeStateStore) -> Result<(), PersistenceError> {
        store.save(
            self.workspace_root.as_deref(),
            self.next_sequence,
            &self.sessions,
            &self.tasks,
            &self.artifacts,
            &self.subagents,
            &self.channel_objects,
            &self.work_items,
            &self.events,
            &self.logs,
            self.scheduler.queued(),
            self.scheduler.wakeups(),
            &self.memory_store.all().into_iter().cloned().collect::<Vec<_>>(),
        )
    }

    pub fn workspace_contracts(&self) -> Option<&WorkspaceContracts> {
        self.workspace_contracts.as_ref()
    }

    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    pub fn skill_registry(&self) -> &SkillRegistry {
        &self.skill_registry
    }

    pub fn provider_catalog(&self) -> &ProviderCatalog {
        &self.provider_catalog
    }

    pub fn scheduler(&self) -> &TaskQueue {
        &self.scheduler
    }

    pub fn memory_store(&self) -> &MemoryStore {
        &self.memory_store
    }

    pub fn task_state(&self, task_id: &str) -> Option<&TaskProjection> {
        self.task_state.get(task_id)
    }

    pub fn remember(&mut self, record: MemoryRecord) {
        self.memory_store.insert(record);
    }

    pub fn render_workspace_memory_projection(&self, updated_at: &str) -> String {
        let records = self
            .memory_store
            .active()
            .into_iter()
            .filter(|record| matches!(record.scope.as_str(), "workspace" | "project"))
            .collect::<Vec<_>>();
        render_memory_markdown(&records, updated_at, "workspace", "Stable Workspace Facts")
    }

    pub fn render_daily_memory_projection(&self, updated_at: &str) -> String {
        let records = self.memory_store.active();
        render_memory_markdown(&records, updated_at, "daily", "Daily Memory Projection")
    }

    pub fn assemble_prompt_context(
        &self,
        task_id: Option<&str>,
        generated_at_ms: u64,
    ) -> Result<PromptContextBundle, RuntimeActionError> {
        assemble_prompt_context(self, task_id, generated_at_ms)
    }

    pub fn write_workspace_memory_projection(
        &self,
        updated_at: &str,
    ) -> Result<FileWriteResult, RuntimeActionError> {
        let workspace_root = self.resolve_workspace_root().to_path_buf();
        self.write_file(FileWriteRequest {
            path: workspace_root.join("MEMORY.md"),
            content: self.render_workspace_memory_projection(updated_at).into_bytes(),
            approval_granted: false,
        })
    }

    pub fn write_daily_memory_projection(
        &self,
        date_slug: &str,
    ) -> Result<FileWriteResult, RuntimeActionError> {
        let workspace_root = self.resolve_workspace_root().to_path_buf();
        self.write_file(FileWriteRequest {
            path: workspace_root
                .join("memory")
                .join("daily")
                .join(format!("{date_slug}.md")),
            content: self.render_daily_memory_projection(date_slug).into_bytes(),
            approval_granted: false,
        })
    }

    pub fn execution_policy(&self) -> WorkspaceExecutionPolicy {
        self.workspace_contracts
            .as_ref()
            .map(|contracts| contracts.execution_policy.clone())
            .unwrap_or_default()
    }

    pub fn plan_exec(&self, command: &str) -> ExecPlan {
        self.execution_policy().evaluate_exec_command(command)
    }

    pub fn execute_exec(&self, request: ExecRequest) -> Result<ExecResult, RuntimeActionError> {
        let plan = self.plan_exec(&request.command_preview());
        run_exec(request, plan, self.resolve_workspace_root())
    }

    pub fn execute_exec_for_task(
        &mut self,
        task_id: &str,
        request: ExecRequest,
        timestamp_ms: u64,
    ) -> Result<ExecResult, RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;

        let plan = self.plan_exec(&request.command_preview());
        let policy_detail = format!("{} (command: {})", plan.reason, request.command_preview());
        let policy_event = self.build_task_event(
            &task,
            match (plan.allowed, plan.requires_approval && !request.approval_granted) {
                (true, false) => DomainEvent::PolicyAllowed {
                    action: "exec".into(),
                    detail: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
                (true, true) => DomainEvent::PolicyApprovalRequired {
                    action: "exec".into(),
                    reason: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
                (false, _) => DomainEvent::PolicyDenied {
                    action: "exec".into(),
                    reason: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
            },
            timestamp_ms,
            EventVisibility::Audit,
        );
        self.append_event(policy_event);

        if !plan.allowed || (plan.requires_approval && !request.approval_granted) {
            return run_exec(request, plan, self.resolve_workspace_root());
        }

        let called = self.build_task_event(
            &task,
            DomainEvent::ToolCalled {
                name: "exec".into(),
            },
            timestamp_ms.saturating_add(1),
            EventVisibility::Ui,
        );
        self.append_event(called);

        match run_exec(request, plan, self.resolve_workspace_root()) {
            Ok(result) => {
                let finished = self.build_task_event(
                    &task,
                    DomainEvent::ToolFinished {
                        name: "exec".into(),
                    },
                    timestamp_ms.saturating_add(2),
                    EventVisibility::Ui,
                );
                self.append_event(finished);

                let heartbeat = self.build_task_event(
                    &task,
                    DomainEvent::TaskHeartbeat {
                        message: Some(format!("exec finished: {}", result.command)),
                    },
                    timestamp_ms.saturating_add(3),
                    EventVisibility::Ui,
                );
                self.append_event(heartbeat);

                Ok(result)
            }
            Err(error) => {
                if !matches!(error, RuntimeActionError::ApprovalRequired { .. }) {
                    let blocked = self.build_task_event(
                        &task,
                        DomainEvent::TaskBlocked {
                            reason: error.to_string(),
                        },
                        timestamp_ms.saturating_add(2),
                        EventVisibility::Ui,
                    );
                    self.append_event(blocked);
                }
                Err(error)
            }
        }
    }

    pub fn plan_file_write(
        &self,
        target_path: &std::path::Path,
        destructive: bool,
    ) -> FileWritePlan {
        let root = self
            .workspace_root
            .as_deref()
            .or_else(|| {
                self.sessions
                    .values()
                    .next()
                    .map(|session| std::path::Path::new(&session.workspace_root))
            })
            .unwrap_or_else(|| std::path::Path::new("."));

        self.execution_policy()
            .evaluate_file_write(root, target_path, destructive)
    }

    pub fn write_file(
        &self,
        request: FileWriteRequest,
    ) -> Result<FileWriteResult, RuntimeActionError> {
        let destructive = request.path.exists();
        let plan = self.plan_file_write(&request.path, destructive);
        run_file_write(request, plan)
    }

    pub fn write_file_for_task(
        &mut self,
        task_id: &str,
        request: FileWriteRequest,
        timestamp_ms: u64,
    ) -> Result<FileWriteResult, RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;

        let destructive = request.path.exists();
        let plan = self.plan_file_write(&request.path, destructive);
        let policy_detail = format!("{} (path: {})", plan.reason, plan.normalized_path.display());
        let policy_event = self.build_task_event(
            &task,
            match (plan.allowed, plan.requires_approval && !request.approval_granted) {
                (true, false) => DomainEvent::PolicyAllowed {
                    action: "filesystem".into(),
                    detail: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
                (true, true) => DomainEvent::PolicyApprovalRequired {
                    action: "filesystem".into(),
                    reason: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
                (false, _) => DomainEvent::PolicyDenied {
                    action: "filesystem".into(),
                    reason: policy_detail.clone(),
                    risk_level: plan.risk_level.clone(),
                },
            },
            timestamp_ms,
            EventVisibility::Audit,
        );
        self.append_event(policy_event);

        if !plan.allowed || (plan.requires_approval && !request.approval_granted) {
            return run_file_write(request, plan);
        }

        let called = self.build_task_event(
            &task,
            DomainEvent::ToolCalled {
                name: "filesystem".into(),
            },
            timestamp_ms.saturating_add(1),
            EventVisibility::Ui,
        );
        self.append_event(called);

        match run_file_write(request, plan) {
            Ok(result) => {
                let finished = self.build_task_event(
                    &task,
                    DomainEvent::ToolFinished {
                        name: "filesystem".into(),
                    },
                    timestamp_ms.saturating_add(2),
                    EventVisibility::Ui,
                );
                self.append_event(finished);

                let heartbeat = self.build_task_event(
                    &task,
                    DomainEvent::TaskHeartbeat {
                        message: Some(format!(
                            "file write finished: {}",
                            result.plan.normalized_path.display()
                        )),
                    },
                    timestamp_ms.saturating_add(3),
                    EventVisibility::Ui,
                );
                self.append_event(heartbeat);

                Ok(result)
            }
            Err(error) => {
                if !matches!(error, RuntimeActionError::ApprovalRequired { .. }) {
                    let blocked = self.build_task_event(
                        &task,
                        DomainEvent::TaskBlocked {
                            reason: error.to_string(),
                        },
                        timestamp_ms.saturating_add(2),
                        EventVisibility::Ui,
                    );
                    self.append_event(blocked);
                }
                Err(error)
            }
        }
    }

    pub fn register_task(&mut self, task: Task) -> EventEnvelope {
        let projection = TaskProjection::from_task(&task);
        let envelope = self.build_task_event(
            &task,
            DomainEvent::TaskCreated {
                initial_phase: None,
                host: task.host.clone(),
            },
            task.created_at_ms,
            EventVisibility::Audit,
        );

        self.task_state.insert(task.id.clone(), projection);
        self.task_attention.entry(task.id.clone()).or_insert(false);
        self.tasks.insert(task.id.clone(), task);
        self.append_event(envelope.clone());
        envelope
    }

    pub fn enqueue_task(
        &mut self,
        task_id: &str,
        enqueued_at_ms: u64,
        background: bool,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        self.scheduler.enqueue(QueuedTask {
            task_id: task.id.clone(),
            priority: task.priority.clone(),
            enqueued_at_ms,
            background,
        });
        self.task_attention.insert(task.id, background);
        Ok(())
    }

    pub fn cancel_task(
        &mut self,
        task_id: &str,
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        self.scheduler.remove_task(task_id);
        self.scheduler.remove_wakeups_for_task(task_id);
        let blocked = self.build_task_event(
            &task,
            DomainEvent::TaskBlocked {
                reason: reason.into(),
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(blocked);
        let cancelled = self.build_task_event(
            &task,
            DomainEvent::TaskCancelled,
            timestamp_ms.saturating_add(1),
            EventVisibility::Ui,
        );
        self.append_event(cancelled);
        Ok(())
    }

    pub fn requeue_task(
        &mut self,
        task_id: &str,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<(), RuntimeActionError> {
        self.scheduler.remove_task(task_id);
        self.enqueue_task(task_id, timestamp_ms, background)
    }

    pub fn cancel_tasks(
        &mut self,
        task_ids: &[String],
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> TaskBatchResult {
        let reason = reason.into();
        let mut result = TaskBatchResult {
            requested: task_ids.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for task_id in task_ids {
            match self.cancel_task(task_id, reason.clone(), timestamp_ms) {
                Ok(()) => result.succeeded.push(task_id.clone()),
                Err(error) => result.failed.push(TaskBatchFailure {
                    task_id: task_id.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn requeue_tasks(
        &mut self,
        task_ids: &[String],
        timestamp_ms: u64,
        background: bool,
    ) -> TaskBatchResult {
        let mut result = TaskBatchResult {
            requested: task_ids.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for task_id in task_ids {
            match self.requeue_task(task_id, timestamp_ms, background) {
                Ok(()) => result.succeeded.push(task_id.clone()),
                Err(error) => result.failed.push(TaskBatchFailure {
                    task_id: task_id.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn sync_channel_host(
        &mut self,
        source_system: &str,
        workspace_id: Option<&str>,
        object_type: &str,
        timestamp_ms: u64,
    ) -> Result<String, RuntimeActionError> {
        let host_key = channel_host_runtime_key(source_system, workspace_id, object_type);
        let connector_id = format!("{source_system}:{object_type}");

        let matching_objects = self
            .channel_objects
            .values()
            .filter(|object| {
                object.source_system == source_system
                    && object.object_type == object_type
                    && object.workspace_id.as_deref() == workspace_id
            })
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();

        if matching_objects.is_empty() {
            return Err(RuntimeActionError::Io(format!(
                "no channel objects matched host `{host_key}`"
            )));
        }

        for object_id in matching_objects {
            let related_task_ids = self
                .work_items
                .values()
                .filter(|work_item| work_item.channel_object_id.as_deref() == Some(object_id.as_str()))
                .filter_map(|work_item| {
                    self.tasks
                        .values()
                        .find(|task| task.work_item_id.as_deref() == Some(work_item.id.as_str()))
                        .map(|task| task.id.clone())
                })
                .collect::<Vec<_>>();

            for task_id in related_task_ids {
                let task = self
                    .tasks
                    .get(&task_id)
                    .cloned()
                    .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.clone()))?;
                let event = self.build_task_event(
                    &task,
                    DomainEvent::ExternalSynced {
                        connector_id: connector_id.clone(),
                    },
                    timestamp_ms,
                    EventVisibility::ExternalSync,
                );
                self.append_event(event);
            }
        }

        self.log_info(format!("synced channel host `{host_key}`"), timestamp_ms);
        Ok(host_key)
    }

    pub fn sync_channel_host_by_key(
        &mut self,
        host_key: &str,
        timestamp_ms: u64,
    ) -> Result<String, RuntimeActionError> {
        let (source_system, workspace_id, object_type) =
            parse_channel_host_runtime_key(host_key)?;
        self.sync_channel_host(
            &source_system,
            workspace_id.as_deref(),
            &object_type,
            timestamp_ms,
        )
    }

    pub fn sync_channel_hosts(
        &mut self,
        host_keys: &[(String, Option<String>, String)],
        timestamp_ms: u64,
    ) -> ConnectorBatchResult {
        let mut result = ConnectorBatchResult {
            requested: host_keys.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for (source_system, workspace_id, object_type) in host_keys {
            match self.sync_channel_host(
                source_system,
                workspace_id.as_deref(),
                object_type,
                timestamp_ms,
            ) {
                Ok(host_key) => result.succeeded.push(host_key),
                Err(error) => result.failed.push(ConnectorBatchFailure {
                    host_key: channel_host_runtime_key(
                        source_system,
                        workspace_id.as_deref(),
                        object_type,
                    ),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn sync_channel_host_keys(
        &mut self,
        host_keys: &[String],
        timestamp_ms: u64,
    ) -> ConnectorBatchResult {
        let mut result = ConnectorBatchResult {
            requested: host_keys.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for host_key in host_keys {
            match self.sync_channel_host_by_key(host_key, timestamp_ms) {
                Ok(host_key) => result.succeeded.push(host_key),
                Err(error) => result.failed.push(ConnectorBatchFailure {
                    host_key: host_key.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn execute_task_center_action(
        &mut self,
        action_key: &str,
        item_keys: &[String],
        reason: Option<String>,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<ClientActionResult, RuntimeActionError> {
        match action_key {
            "cancel_selected" => {
                let reason =
                    reason.ok_or_else(|| RuntimeActionError::MissingActionReason {
                        action_key: action_key.into(),
                    })?;
                Ok(map_task_batch_result(
                    action_key,
                    self.cancel_tasks(item_keys, reason, timestamp_ms),
                ))
            }
            "requeue_selected" => Ok(map_task_batch_result(
                action_key,
                self.requeue_tasks(item_keys, timestamp_ms, background),
            )),
            other => Err(RuntimeActionError::UnsupportedClientAction {
                action_key: other.into(),
            }),
        }
    }

    pub fn execute_approval_center_action(
        &mut self,
        action_key: &str,
        item_keys: &[String],
        reason: Option<String>,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<ClientActionResult, RuntimeActionError> {
        match action_key {
            "approve_selected" => Ok(map_approval_batch_result(
                action_key,
                self.approve_pending_approvals(item_keys, timestamp_ms, background),
            )),
            "reject_selected" => {
                let reason =
                    reason.ok_or_else(|| RuntimeActionError::MissingActionReason {
                        action_key: action_key.into(),
                    })?;
                Ok(map_approval_batch_result(
                    action_key,
                    self.reject_pending_approvals(item_keys, reason, timestamp_ms),
                ))
            }
            other => Err(RuntimeActionError::UnsupportedClientAction {
                action_key: other.into(),
            }),
        }
    }

    pub fn execute_connector_center_action(
        &mut self,
        action_key: &str,
        item_keys: &[String],
        timestamp_ms: u64,
    ) -> Result<ClientActionResult, RuntimeActionError> {
        match action_key {
            "sync_selected_hosts" => Ok(map_connector_batch_result(
                action_key,
                self.sync_channel_host_keys(item_keys, timestamp_ms),
            )),
            "sync_all_hosts" => {
                let host_keys = self
                    .client_surface_snapshot(timestamp_ms)
                    .channel_hosts
                    .into_iter()
                    .map(|item| item.host_key)
                    .collect::<Vec<_>>();
                Ok(map_connector_batch_result(
                    action_key,
                    self.sync_channel_host_keys(&host_keys, timestamp_ms),
                ))
            }
            other => Err(RuntimeActionError::UnsupportedClientAction {
                action_key: other.into(),
            }),
        }
    }

    pub fn execute_client_action(
        &mut self,
        request: ClientActionRequest,
    ) -> Result<ClientActionResult, RuntimeActionError> {
        match request.center {
            ClientCenterKind::Task => self.execute_task_center_action(
                &request.action_key,
                &request.item_keys,
                request.reason,
                request.timestamp_ms,
                request.background,
            ),
            ClientCenterKind::Approval => self.execute_approval_center_action(
                &request.action_key,
                &request.item_keys,
                request.reason,
                request.timestamp_ms,
                request.background,
            ),
            ClientCenterKind::Connector => self.execute_connector_center_action(
                &request.action_key,
                &request.item_keys,
                request.timestamp_ms,
            ),
        }
    }

    pub fn run_task_script(
        &mut self,
        task_id: &str,
        script: TaskScript,
        started_at_ms: u64,
    ) -> Result<TaskRunReport, RuntimeActionError> {
        let action_count = script.actions.len();
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;

        let started = self.build_task_event(&task, DomainEvent::TaskStarted, started_at_ms, EventVisibility::Ui);
        self.append_event(started);

        let mut timestamp = started_at_ms.saturating_add(1);
        let mut waiting_for_wakeup = false;
        let mut completed = false;
        let mut failed = false;
        let mut replanned_actions = 0_usize;

        for (action_index, action) in script.actions.into_iter().enumerate() {
            match action {
                TaskAction::Progress {
                    phase,
                    current_step,
                    total_steps,
                    message,
                } => {
                    let event = self.build_task_event(
                        &task,
                        DomainEvent::TaskProgress {
                            phase,
                            current_step,
                            total_steps,
                            progress_text: message,
                        },
                        timestamp,
                        EventVisibility::Ui,
                    );
                    self.append_event(event);
                }
                TaskAction::Heartbeat { message } => {
                    let event = self.build_task_event(
                        &task,
                        DomainEvent::TaskHeartbeat { message },
                        timestamp,
                        EventVisibility::Ui,
                    );
                    self.append_event(event);
                }
                TaskAction::Exec { request } => {
                    match self.execute_exec_for_task(task_id, request, timestamp) {
                        Ok(_) => {}
                        Err(RuntimeActionError::ApprovalRequired { reason }) => {
                            let checkpoint = resume_checkpoint_for_action(action_index, "exec");
                            let event = self.build_task_event(
                                &task,
                                DomainEvent::TaskWaiting {
                                    kind: crate::state::TaskWaitKind::Approval,
                                    reason,
                                    resume_checkpoint: Some(checkpoint.clone()),
                                },
                                timestamp,
                                EventVisibility::Ui,
                            );
                            self.append_event(event);
                            return Ok(TaskRunReport {
                                task_id: task_id.to_string(),
                                action_count,
                                completed: false,
                                waiting_for_wakeup: false,
                                waiting_for_approval: true,
                                resume_checkpoint: Some(checkpoint),
                                failed: false,
                                replanned_actions,
                            });
                        }
                        Err(RuntimeActionError::ReplanSuggested { reason }) => {
                            replanned_actions += 1;
                            let event = self.build_task_event(
                                &task,
                                DomainEvent::TaskProgress {
                                    phase: Some("replan".into()),
                                    current_step: None,
                                    total_steps: None,
                                    progress_text: format!("replan suggested: {reason}"),
                                },
                                timestamp,
                                EventVisibility::Ui,
                            );
                            self.append_event(event);
                        }
                        Err(error) => return Err(map_action_error(error)),
                    }
                }
                TaskAction::WriteFile { request } => {
                    match self.write_file_for_task(task_id, request, timestamp) {
                        Ok(_) => {}
                        Err(RuntimeActionError::ApprovalRequired { reason }) => {
                            let checkpoint =
                                resume_checkpoint_for_action(action_index, "write_file");
                            let event = self.build_task_event(
                                &task,
                                DomainEvent::TaskWaiting {
                                    kind: crate::state::TaskWaitKind::Approval,
                                    reason,
                                    resume_checkpoint: Some(checkpoint.clone()),
                                },
                                timestamp,
                                EventVisibility::Ui,
                            );
                            self.append_event(event);
                            return Ok(TaskRunReport {
                                task_id: task_id.to_string(),
                                action_count,
                                completed: false,
                                waiting_for_wakeup: false,
                                waiting_for_approval: true,
                                resume_checkpoint: Some(checkpoint),
                                failed: false,
                                replanned_actions,
                            });
                        }
                        Err(RuntimeActionError::ReplanSuggested { reason }) => {
                            replanned_actions += 1;
                            let event = self.build_task_event(
                                &task,
                                DomainEvent::TaskProgress {
                                    phase: Some("replan".into()),
                                    current_step: None,
                                    total_steps: None,
                                    progress_text: format!("replan suggested: {reason}"),
                                },
                                timestamp,
                                EventVisibility::Ui,
                            );
                            self.append_event(event);
                        }
                        Err(error) => return Err(map_action_error(error)),
                    }
                }
                TaskAction::Remember { record } => {
                    self.remember(record);
                    let event = self.build_task_event(
                        &task,
                        DomainEvent::TaskHeartbeat {
                            message: Some("memory updated".into()),
                        },
                        timestamp,
                        EventVisibility::Ui,
                    );
                    self.append_event(event);
                }
                TaskAction::ScheduleWakeup { wake_at_ms, reason } => {
                    self.schedule_task_wakeup(
                        task_id,
                        wake_at_ms,
                        reason,
                        Some(resume_checkpoint_for_action(action_index, "scheduled_resume")),
                        timestamp,
                    )?;
                    waiting_for_wakeup = true;
                }
                TaskAction::Complete { summary } => {
                    let event = self.build_task_event(
                        &task,
                        DomainEvent::TaskCompleted { summary },
                        timestamp,
                        EventVisibility::Ui,
                    );
                    self.append_event(event);
                    completed = true;
                }
                TaskAction::Fail { error } => {
                    let event = self.build_task_event(
                        &task,
                        DomainEvent::TaskFailed { error },
                        timestamp,
                        EventVisibility::Ui,
                    );
                    self.append_event(event);
                    failed = true;
                }
            }
            if completed || failed {
                break;
            }
            timestamp = timestamp.saturating_add(3);
        }

        if script.auto_complete && !completed && !failed && !waiting_for_wakeup {
            let event = self.build_task_event(
                &task,
                DomainEvent::TaskCompleted { summary: None },
                timestamp,
                EventVisibility::Ui,
            );
            self.append_event(event);
            completed = true;
        }

        Ok(TaskRunReport {
            task_id: task_id.to_string(),
            action_count,
            completed,
            waiting_for_wakeup,
            waiting_for_approval: false,
            resume_checkpoint: None,
            failed,
            replanned_actions,
        })
    }

    pub fn start_next_queued_task(
        &mut self,
        timestamp_ms: u64,
    ) -> Result<Option<String>, RuntimeActionError> {
        let Some(queued) = self.scheduler.dequeue_next() else {
            return Ok(None);
        };
        let task = self
            .tasks
            .get(&queued.task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(queued.task_id.clone()))?;
        let event = self.build_task_event(&task, DomainEvent::TaskStarted, timestamp_ms, EventVisibility::Ui);
        self.append_event(event);
        Ok(Some(task.id))
    }

    pub fn process_next_queued_script(
        &mut self,
        script: TaskScript,
        started_at_ms: u64,
    ) -> Result<Option<TaskRunReport>, RuntimeActionError> {
        let Some(queued) = self.scheduler.dequeue_next() else {
            return Ok(None);
        };
        let report = self.run_task_script(&queued.task_id, script, started_at_ms)?;
        Ok(Some(report))
    }

    pub fn schedule_task_wakeup(
        &mut self,
        task_id: &str,
        wake_at_ms: u64,
        reason: impl Into<String>,
        resume_checkpoint: Option<String>,
        timestamp_ms: u64,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let reason = reason.into();
        self.scheduler.schedule_wakeup(ScheduledWakeup {
            task_id: task.id.clone(),
            wake_at_ms,
            reason: reason.clone(),
        });
        let waiting = self.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Schedule,
                reason,
                resume_checkpoint,
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(waiting);
        Ok(())
    }

    pub fn process_due_wakeups(
        &mut self,
        now_ms: u64,
    ) -> Result<Vec<String>, RuntimeActionError> {
        let due = self.scheduler.drain_due_wakeups(now_ms);
        let mut resumed = Vec::new();
        for wakeup in due {
            let task = self
                .tasks
                .get(&wakeup.task_id)
                .cloned()
                .ok_or_else(|| RuntimeActionError::UnknownTask(wakeup.task_id.clone()))?;
            let resumed_event =
                self.build_task_event(&task, DomainEvent::TaskResumed, now_ms, EventVisibility::Ui);
            self.append_event(resumed_event);
            self.scheduler.enqueue(QueuedTask {
                task_id: task.id.clone(),
                priority: task.priority.clone(),
                enqueued_at_ms: now_ms,
                background: true,
            });
            self.task_attention.insert(task.id.clone(), true);
            resumed.push(task.id);
        }
        Ok(resumed)
    }

    pub fn resume_task_from_checkpoint(
        &mut self,
        task_id: &str,
        checkpoint: &str,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let projection = self
            .task_state
            .get(task_id)
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let expected = projection
            .resume_checkpoint
            .clone()
            .ok_or_else(|| RuntimeActionError::InvalidResumeCheckpoint {
                reason: "task does not currently expose a resume checkpoint".into(),
            })?;
        if expected != checkpoint {
            return Err(RuntimeActionError::InvalidResumeCheckpoint {
                reason: format!("expected `{expected}` but received `{checkpoint}`"),
            });
        }

        let resumed_event =
            self.build_task_event(&task, DomainEvent::TaskResumed, timestamp_ms, EventVisibility::Ui);
        self.append_event(resumed_event);
        self.scheduler.enqueue(QueuedTask {
            task_id: task.id.clone(),
            priority: task.priority.clone(),
            enqueued_at_ms: timestamp_ms,
            background,
        });
        self.task_attention.insert(task.id.clone(), background);
        self.log_info(
            format!("task `{}` resumed from checkpoint `{checkpoint}`", task.id),
            timestamp_ms,
        );
        Ok(())
    }

    pub fn approve_task_checkpoint(
        &mut self,
        task_id: &str,
        checkpoint: &str,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let projection = self
            .task_state
            .get(task_id)
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let action = projection
            .approval_action
            .clone()
            .unwrap_or_else(|| "unknown".into());
        let risk_level = projection
            .approval_risk
            .clone()
            .unwrap_or(crate::policy::ApprovalRiskLevel::Low);
        let approved = self.build_task_event(
            &task,
            DomainEvent::PolicyApproved {
                action,
                checkpoint: checkpoint.to_string(),
                risk_level,
                decision_source: "user".into(),
                resolved_by: "user".into(),
            },
            timestamp_ms,
            EventVisibility::Audit,
        );
        self.append_event(approved);
        self.resume_task_from_checkpoint(task_id, checkpoint, timestamp_ms, background)
    }

    pub fn approve_pending_approval(
        &mut self,
        approval_id: &str,
        timestamp_ms: u64,
        background: bool,
    ) -> Result<(), RuntimeActionError> {
        let (task_id, checkpoint) = self
            .find_pending_approval(approval_id)
            .ok_or_else(|| RuntimeActionError::InvalidResumeCheckpoint {
                reason: format!("unknown pending approval `{approval_id}`"),
            })?;
        self.approve_task_checkpoint(&task_id, &checkpoint, timestamp_ms, background)
    }

    pub fn approve_pending_approvals(
        &mut self,
        approval_ids: &[String],
        timestamp_ms: u64,
        background: bool,
    ) -> ApprovalBatchResult {
        let mut result = ApprovalBatchResult {
            requested: approval_ids.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for approval_id in approval_ids {
            match self.approve_pending_approval(approval_id, timestamp_ms, background) {
                Ok(()) => result.succeeded.push(approval_id.clone()),
                Err(error) => result.failed.push(ApprovalBatchFailure {
                    approval_id: approval_id.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn reject_task_checkpoint(
        &mut self,
        task_id: &str,
        checkpoint: &str,
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let projection = self
            .task_state
            .get(task_id)
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let expected = projection
            .resume_checkpoint
            .clone()
            .ok_or_else(|| RuntimeActionError::InvalidResumeCheckpoint {
                reason: "task does not currently expose a resume checkpoint".into(),
            })?;
        if expected != checkpoint {
            return Err(RuntimeActionError::InvalidResumeCheckpoint {
                reason: format!("expected `{expected}` but received `{checkpoint}`"),
            });
        }

        let reason = format!("approval rejected: {}", reason.into());
        let approved_action = projection
            .approval_action
            .clone()
            .unwrap_or_else(|| "unknown".into());
        let risk_level = projection
            .approval_risk
            .clone()
            .unwrap_or(crate::policy::ApprovalRiskLevel::Low);
        let rejected = self.build_task_event(
            &task,
            DomainEvent::PolicyRejected {
                action: approved_action,
                checkpoint: checkpoint.to_string(),
                reason: reason.clone(),
                risk_level,
                decision_source: "user".into(),
                resolved_by: "user".into(),
            },
            timestamp_ms,
            EventVisibility::Audit,
        );
        self.append_event(rejected);
        let blocked = self.build_task_event(
            &task,
            DomainEvent::TaskBlocked {
                reason: reason.clone(),
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(blocked);
        let cancelled = self.build_task_event(
            &task,
            DomainEvent::TaskCancelled,
            timestamp_ms.saturating_add(1),
            EventVisibility::Ui,
        );
        self.append_event(cancelled);
        self.log_warn(reason, Some(task.id), timestamp_ms);
        Ok(())
    }

    pub fn reject_pending_approval(
        &mut self,
        approval_id: &str,
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> Result<(), RuntimeActionError> {
        let (task_id, checkpoint) = self
            .find_pending_approval(approval_id)
            .ok_or_else(|| RuntimeActionError::InvalidResumeCheckpoint {
                reason: format!("unknown pending approval `{approval_id}`"),
            })?;
        self.reject_task_checkpoint(&task_id, &checkpoint, reason, timestamp_ms)
    }

    pub fn reject_pending_approvals(
        &mut self,
        approval_ids: &[String],
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> ApprovalBatchResult {
        let rejection_reason = reason.into();
        let mut result = ApprovalBatchResult {
            requested: approval_ids.len() as u32,
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for approval_id in approval_ids {
            match self.reject_pending_approval(
                approval_id,
                rejection_reason.clone(),
                timestamp_ms,
            ) {
                Ok(()) => result.succeeded.push(approval_id.clone()),
                Err(error) => result.failed.push(ApprovalBatchFailure {
                    approval_id: approval_id.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        result
    }

    pub fn append_event(&mut self, envelope: EventEnvelope) {
        if let Some(task_id) = &envelope.task_id {
            if let Some(projection) = self.task_state.get_mut(task_id) {
                projection.apply_event(&envelope);
            }
        }
        self.events.push(envelope);
    }

    pub fn append_log(
        &mut self,
        level: RuntimeLogLevel,
        message: impl Into<String>,
        task_id: Option<String>,
        timestamp_ms: u64,
    ) -> RuntimeLogEntry {
        self.next_sequence += 1;
        let entry = RuntimeLogEntry {
            id: format!("log-{}", self.next_sequence),
            level,
            message: message.into(),
            task_id,
            timestamp_ms,
        };
        self.logs.push(entry.clone());
        entry
    }

    pub fn log_debug(&mut self, message: impl Into<String>, timestamp_ms: u64) -> RuntimeLogEntry {
        self.append_log(RuntimeLogLevel::Debug, message, None, timestamp_ms)
    }

    pub fn log_info(&mut self, message: impl Into<String>, timestamp_ms: u64) -> RuntimeLogEntry {
        self.append_log(RuntimeLogLevel::Info, message, None, timestamp_ms)
    }

    pub fn log_warn(
        &mut self,
        message: impl Into<String>,
        task_id: Option<String>,
        timestamp_ms: u64,
    ) -> RuntimeLogEntry {
        self.append_log(RuntimeLogLevel::Warn, message, task_id, timestamp_ms)
    }

    pub fn log_error(
        &mut self,
        message: impl Into<String>,
        task_id: Option<String>,
        timestamp_ms: u64,
    ) -> RuntimeLogEntry {
        self.append_log(RuntimeLogLevel::Error, message, task_id, timestamp_ms)
    }

    pub fn heartbeat_all(
        &mut self,
        now_ms: u64,
        degrade_after_ms: u64,
        stall_after_ms: u64,
    ) {
        for projection in self.task_state.values_mut() {
            projection.apply_watchdog(now_ms, degrade_after_ms, stall_after_ms);
        }
    }

    pub fn projection(&self, task_id: &str) -> Option<&TaskProjection> {
        self.task_state.get(task_id)
    }

    pub fn task_board_entries(&self) -> Vec<TaskBoardEntry> {
        let mut entries = self
            .task_state
            .values()
            .map(|projection| {
                let is_queued = self
                    .scheduler
                    .queued()
                    .iter()
                    .any(|queued| queued.task_id == projection.task_id);
                let has_background_subagent = self.subagents.values().any(|subagent| {
                    subagent.parent_task_id == projection.task_id
                        && subagent.background
                        && matches!(
                            subagent.status,
                            SubagentStatus::Queued | SubagentStatus::Running | SubagentStatus::Waiting
                        )
                });
                let lane = if self
                    .task_attention
                    .get(&projection.task_id)
                    .copied()
                    .unwrap_or(false)
                || has_background_subagent
                {
                    TaskAttentionLane::Background
                } else {
                    TaskAttentionLane::Foreground
                };
                build_task_board_entry(
                    projection,
                    host_label_for_task_host(&projection.host),
                    lane,
                    is_queued,
                )
            })
            .collect::<Vec<_>>();

        entries.sort_by(|left, right| {
            lane_rank(&left.attention_lane)
                .cmp(&lane_rank(&right.attention_lane))
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        entries
    }

    pub fn task_board_snapshot(&self, generated_at_ms: u64) -> TaskBoardSnapshot {
        build_task_board_snapshot(generated_at_ms, self.task_board_entries())
    }

    pub fn client_surface_snapshot(&self, generated_at_ms: u64) -> ClientSurfaceSnapshot {
        let task_board = self.task_board_snapshot(generated_at_ms);
        let sessions = build_session_surface_entries(
            self.sessions
                .values()
                .map(|session| (session.id.clone(), session.title.clone()))
                .collect(),
            &task_board.entries,
        );
        let channel_sources = build_channel_source_counts(
            self.channel_objects
                .values()
                .map(|object| object.source_system.clone())
                .collect(),
        );
        let providers = build_provider_surface_entries(
            self.provider_catalog
                .all()
                .into_iter()
                .map(|provider| {
                    (
                        provider.profile.clone(),
                        provider.adapter.provider_id.clone(),
                        provider.adapter.protocol.clone(),
                        provider.adapter.api_family.clone(),
                        provider.adapter.header_policy.clone(),
                        provider.adapter.models.len(),
                        provider.adapter.capabilities.clone(),
                    )
                })
                .collect(),
        );
        let waiting_buckets = build_wait_bucket_counts(&task_board.entries);
        let approval_queue = build_approval_surface_entries(&task_board.entries);
        let approval_history = build_approval_history_entries(
            self.events
                .iter()
                .filter_map(|event| {
                    let task_id = event.task_id.clone()?;
                    let title = self
                        .tasks
                        .get(&task_id)
                        .map(|task| task.title.clone())
                        .unwrap_or_else(|| task_id.clone());
                    match &event.event {
                        DomainEvent::PolicyApproved {
                            action,
                            checkpoint,
                            risk_level,
                            decision_source,
                            resolved_by,
                        } => Some((
                            approval_id_for(&task_id, Some(action), Some(checkpoint)),
                            task_id,
                            title,
                            "approved".into(),
                            action.clone(),
                            approval_risk_label(risk_level),
                            decision_source.clone(),
                            resolved_by.clone(),
                            event.timestamp_ms,
                            None,
                        )),
                        DomainEvent::PolicyRejected {
                            action,
                            checkpoint,
                            risk_level,
                            reason,
                            decision_source,
                            resolved_by,
                        } => Some((
                            approval_id_for(&task_id, Some(action), Some(checkpoint)),
                            task_id,
                            title,
                            "rejected".into(),
                            action.clone(),
                            approval_risk_label(risk_level),
                            decision_source.clone(),
                            resolved_by.clone(),
                            event.timestamp_ms,
                            Some(reason.clone()),
                        )),
                        _ => None,
                    }
                })
                .collect(),
        );
        let approval_summary =
            build_approval_center_summary(&approval_queue, &approval_history);
        let approval_groups =
            build_approval_group_buckets(&approval_queue, &approval_history);
        let approval_view_presets =
            build_approval_view_presets(&approval_summary, &approval_history);
        let (approval_history_window, approval_history) =
            apply_approval_history_window(approval_history, DEFAULT_APPROVAL_HISTORY_LIMIT);
        let work_items = build_work_item_surface_entries(
            self.work_items
                .values()
                .map(|work_item| {
                    (
                        work_item.id.clone(),
                        work_item.source_system.clone(),
                        work_item.status.clone(),
                        work_item.summary.clone(),
                        work_item.channel_object_id.clone(),
                    )
                })
                .collect(),
        );
        let channel_inbox = build_channel_inbox_entries(
            self.work_items
                .values()
                .filter_map(|work_item| {
                    let channel_object = work_item
                        .channel_object_id
                        .as_ref()
                        .and_then(|object_id| self.channel_objects.get(object_id))?;
                    let linked_task = self
                        .tasks
                        .values()
                        .find(|task| task.work_item_id.as_deref() == Some(work_item.id.as_str()));
                    let linked_task_id = linked_task.map(|task| task.id.clone());
                    let linked_projection = linked_task_id
                        .as_ref()
                        .and_then(|task_id| self.task_state.get(task_id));
                    let connector = ConnectorSample;
                    let host_metadata = connector.host_metadata_for(channel_object);
                    Some((
                        work_item.id.clone(),
                        channel_object.source_system.clone(),
                        channel_object.object_type.clone(),
                        channel_host_kind_label(&host_metadata.host_kind),
                        host_metadata.display_name,
                        connector_sync_mode_label(&host_metadata.sync_mode),
                        host_capability_labels(
                            host_metadata.supports_task_creation,
                            host_metadata.supports_status_sync,
                            host_metadata.supports_comment_sync,
                            host_metadata.supports_attachment_export,
                        ),
                        host_metadata.requires_bidirectional_binding,
                        channel_object.external_id.clone(),
                        channel_object.workspace_id.clone(),
                        work_item.status.clone(),
                        work_item.summary.clone(),
                        linked_task_id,
                        linked_projection.map(|projection| projection.lifecycle.clone()),
                        linked_projection.and_then(|projection| projection.waiting_on.clone()),
                        linked_projection.map(|projection| {
                            matches!(
                                projection.lifecycle,
                                crate::state::TaskLifecycleState::Waiting
                                    | crate::state::TaskLifecycleState::Failed
                            ) || matches!(
                                projection.health,
                                crate::state::TaskHealthState::Degraded
                                    | crate::state::TaskHealthState::Stalled
                            )
                        }).unwrap_or(false),
                        host_metadata.supports_writeback,
                        host_metadata.supports_background_sync,
                    ))
                })
                .collect(),
        );
        let channel_hosts = build_channel_host_surface_entries(&channel_inbox);
        build_client_surface_snapshot(
            generated_at_ms,
            task_board,
            sessions,
            channel_sources,
            providers,
            waiting_buckets,
            work_items,
            approval_summary,
            approval_view_presets,
            approval_queue,
            approval_history_window,
            approval_history,
            approval_groups,
            channel_hosts,
            channel_inbox,
            self.work_items.len() as u32,
        )
    }

    pub fn client_api_snapshot(&self, generated_at_ms: u64) -> ClientApiSnapshot {
        build_client_api_snapshot(self.client_surface_snapshot(generated_at_ms))
    }

    pub fn session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn channel_object(&self, object_id: &str) -> Option<&ChannelObject> {
        self.channel_objects.get(object_id)
    }

    pub fn work_item(&self, work_item_id: &str) -> Option<&WorkItem> {
        self.work_items.get(work_item_id)
    }

    pub fn task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    pub fn subagent(&self, subagent_id: &str) -> Option<&Subagent> {
        self.subagents.get(subagent_id)
    }

    pub fn subagents_for_task(&self, task_id: &str) -> Vec<&Subagent> {
        self.subagents
            .values()
            .filter(|subagent| subagent.parent_task_id == task_id)
            .collect()
    }

    pub fn artifact(&self, artifact_id: &str) -> Option<&Artifact> {
        self.artifacts.get(artifact_id)
    }

    pub fn artifacts_for_task(&self, task_id: &str) -> Vec<&Artifact> {
        self.artifacts
            .values()
            .filter(|artifact| artifact.task_id == task_id)
            .collect()
    }

    pub fn events(&self) -> &[EventEnvelope] {
        &self.events
    }

    pub fn logs(&self) -> &[RuntimeLogEntry] {
        &self.logs
    }

    pub fn build_task_event(
        &mut self,
        task: &Task,
        event: DomainEvent,
        timestamp_ms: u64,
        visibility: EventVisibility,
    ) -> EventEnvelope {
        self.next_sequence += 1;
        EventEnvelope::for_task(
            format!("event-{}", self.next_sequence),
            task.id.clone(),
            task.session_id.clone(),
            self.next_sequence,
            timestamp_ms,
            EventSource::Runtime,
            visibility,
            event,
        )
    }

    pub fn promote_task_to_complex_mode(
        &mut self,
        task_id: &str,
        timestamp_ms: u64,
        reason: impl Into<String>,
    ) -> Result<(), RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let reason = reason.into();
        let event = self.build_task_event(
            &task,
            DomainEvent::TaskWorkflowModeChanged {
                mode: TaskWorkflowMode::Complex,
                reason: reason.clone(),
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(event);
        self.log_info(format!("task `{task_id}` promoted to complex mode: {reason}"), timestamp_ms);
        Ok(())
    }

    pub fn task_diagnostic_report(
        &self,
        task_id: &str,
    ) -> Result<TaskDiagnosticReport, RuntimeActionError> {
        let projection = self
            .task_state
            .get(task_id)
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        Ok(build_task_diagnostic_report(
            projection,
            &self.events,
            &self.logs,
        ))
    }

    pub fn spawn_subagent(
        &mut self,
        parent_task_id: &str,
        subagent_id: impl Into<String>,
        role: impl Into<String>,
        background: bool,
        timestamp_ms: u64,
    ) -> Result<Subagent, RuntimeActionError> {
        let task = self
            .tasks
            .get(parent_task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(parent_task_id.to_string()))?;
        let subagent = Subagent {
            id: subagent_id.into(),
            parent_task_id: parent_task_id.to_string(),
            role: role.into(),
            status: SubagentStatus::Queued,
            detail: None,
            background,
            created_at_ms: timestamp_ms,
            updated_at_ms: timestamp_ms,
        };
        self.subagents.insert(subagent.id.clone(), subagent.clone());
        let event = self.build_task_event(
            &task,
            DomainEvent::SubagentSpawned {
                subagent_id: subagent.id.clone(),
                role: subagent.role.clone(),
                background: subagent.background,
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(event);
        self.log_info(
            format!(
                "subagent `{}` spawned for task `{}` as {}",
                subagent.id, parent_task_id, subagent.role
            ),
            timestamp_ms,
        );
        Ok(subagent)
    }

    pub fn update_subagent_status(
        &mut self,
        subagent_id: &str,
        status: SubagentStatus,
        detail: Option<String>,
        timestamp_ms: u64,
    ) -> Result<(), RuntimeActionError> {
        let parent_task_id = {
            let subagent = self
                .subagents
                .get_mut(subagent_id)
                .ok_or_else(|| RuntimeActionError::Io(format!("unknown subagent `{subagent_id}`")))?;
            subagent.status = status.clone();
            subagent.detail = detail.clone();
            subagent.updated_at_ms = timestamp_ms;
            subagent.parent_task_id.clone()
        };

        let task = self
            .tasks
            .get(&parent_task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(parent_task_id.clone()))?;
        let event = self.build_task_event(
            &task,
            DomainEvent::SubagentUpdated {
                subagent_id: subagent_id.to_string(),
                status: status.clone(),
                detail: detail.clone(),
            },
            timestamp_ms,
            EventVisibility::Ui,
        );
        self.append_event(event);
        self.log_info(
            format!("subagent `{subagent_id}` updated to {:?}", status),
            timestamp_ms,
        );
        Ok(())
    }

    fn resolve_workspace_root(&self) -> &std::path::Path {
        self.workspace_root
            .as_deref()
            .or_else(|| {
                self.sessions
                    .values()
                    .next()
                    .map(|session| std::path::Path::new(&session.workspace_root))
            })
            .unwrap_or_else(|| std::path::Path::new("."))
    }

    fn rebuild_projections(&mut self) {
        self.task_state = self
            .tasks
            .values()
            .map(|task| (task.id.clone(), TaskProjection::from_task(task)))
            .collect();
    }

    fn rebuild_task_attention(&mut self) {
        self.task_attention = self
            .tasks
            .keys()
            .map(|task_id| {
                let background = self
                    .scheduler
                    .queued()
                    .iter()
                    .any(|queued| queued.task_id == *task_id && queued.background)
                    || self.subagents.values().any(|subagent| {
                        subagent.parent_task_id == *task_id
                            && subagent.background
                            && matches!(
                                subagent.status,
                                SubagentStatus::Queued
                                    | SubagentStatus::Running
                                    | SubagentStatus::Waiting
                            )
                    });
                (task_id.clone(), background)
            })
            .collect();
    }

    pub fn ingest_connector_envelope(&mut self, envelope: ConnectorEnvelope) {
        self.channel_objects
            .insert(envelope.channel_object.id.clone(), envelope.channel_object);
        self.work_items
            .insert(envelope.work_item.id.clone(), envelope.work_item);
    }

    pub fn cache_artifact(
        &mut self,
        task_id: &str,
        artifact_id: impl Into<String>,
        name: impl Into<String>,
        content: &[u8],
        now_ms: u64,
        ttl_ms: Option<u64>,
    ) -> Result<Artifact, RuntimeActionError> {
        let task = self
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        let artifact = stage_artifact(
            self.resolve_workspace_root(),
            artifact_id,
            task.id.clone(),
            name,
            content,
            now_ms,
            ttl_ms,
        )?;
        self.artifacts.insert(artifact.id.clone(), artifact.clone());
        Ok(artifact)
    }

    pub fn cleanup_expired_artifact_cache(
        &mut self,
        now_ms: u64,
    ) -> Result<Vec<String>, RuntimeActionError> {
        cleanup_expired_artifacts(&mut self.artifacts, now_ms)
    }

    pub fn export_artifact_to(
        &self,
        artifact_id: &str,
        destination_path: &std::path::Path,
    ) -> Result<std::path::PathBuf, RuntimeActionError> {
        let artifact = self
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| RuntimeActionError::Io(format!("unknown artifact `{artifact_id}`")))?;
        export_artifact(artifact, destination_path)
    }
}

fn lane_rank(value: &TaskAttentionLane) -> u8 {
    match value {
        TaskAttentionLane::Foreground => 0,
        TaskAttentionLane::Background => 1,
    }
}

fn resume_checkpoint_for_action(action_index: usize, action_label: &str) -> String {
    format!("action:{action_index}:{action_label}")
}

fn parse_channel_host_runtime_key(
    host_key: &str,
) -> Result<(String, Option<String>, String), RuntimeActionError> {
    let parts = host_key.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [source_system, object_type] => Ok((
            (*source_system).to_string(),
            None,
            (*object_type).to_string(),
        )),
        [source_system, workspace_id, object_type] => Ok((
            (*source_system).to_string(),
            Some((*workspace_id).to_string()),
            (*object_type).to_string(),
        )),
        _ => Err(RuntimeActionError::InvalidConnectorHostKey {
            reason: format!("unsupported host key `{host_key}`"),
        }),
    }
}

impl TaskLoopRuntime {
    fn find_pending_approval(&self, approval_id: &str) -> Option<(String, String)> {
        self.task_state.values().find_map(|projection| {
            if !matches!(projection.waiting_on, Some(crate::state::TaskWaitKind::Approval)) {
                return None;
            }
            let checkpoint = projection.resume_checkpoint.as_ref()?;
            let derived = approval_id_for(
                &projection.task_id,
                projection.approval_action.as_deref(),
                Some(checkpoint),
            );
            if derived == approval_id {
                Some((projection.task_id.clone(), checkpoint.clone()))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        connector::{ChannelObjectPayload, ConnectorSample},
        contracts::WorkspaceLayout,
        execution::{ExecRequest, FileWriteRequest, RuntimeActionError},
        event::{DomainEvent, EventVisibility},
        loop_runner::{TaskAction, TaskScript},
        model::{Session, SubagentStatus, Task, TaskHost, TaskKind, TaskPriority},
        persistence::RuntimeStateStore,
        runtime::TaskLoopRuntime,
        presentation::TaskAttentionLane,
        state::TaskLifecycleState,
    };

    #[test]
    fn runtime_registers_task_and_projects_state() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });

        let task = Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Build skeleton".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        };

        runtime.register_task(task.clone());
        let started = runtime.build_task_event(&task, DomainEvent::TaskStarted, 2_000, EventVisibility::Ui);
        runtime.append_event(started);

        let projection = runtime.projection("task-1").unwrap();
        assert_eq!(projection.lifecycle, TaskLifecycleState::Running);
    }

    #[test]
    fn runtime_can_load_workspace_contracts() {
        let temp_dir = create_temp_workspace("runtime-contracts");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("skills").join("demo")).unwrap();
        fs::create_dir_all(temp_dir.join("config")).unwrap();
        fs::write(
            temp_dir.join("skills").join("demo").join("SKILL.md"),
            r#"---
kind: skill
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
name: Demo Skill
description: Test loader
mode: both
safe_for_background: false
requires_tools:
  - exec
requires_env: []
requires_models: []
requires_os: []
requires_bins: []
requires_any_bins: []
requires_config: []
attachment_types: []
modes:
  - mixed
tags:
  - demo
---

Demo body
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("config").join("providers.toml"),
            r#"[default]
provider_id = "openai_compatible_default"
base_url = "https://api.example.com/v1"
protocol = "openai-compatible"
api_family = "responses"
auth = "bearer"
header_policy = "strict"
user_agent_mode = "runtime-default"
models = ["gpt-4.1-mini"]
capabilities = ["streaming", "tool-calling"]
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        let contracts = runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();

        assert!(contracts.agents.is_some());
        assert_eq!(contracts.skills.len(), 1);
        assert!(runtime.tool_registry().get("exec").is_some());
        assert!(runtime.skill_registry().get("workspace_maintenance").is_some());
        assert!(runtime.skill_registry().get("demo").is_some());
        assert!(runtime.provider_catalog().get("default").is_some());
        assert!(runtime.execution_policy().backup_before_write);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_exposes_exec_and_file_write_plans() {
        let temp_dir = create_temp_workspace("runtime-policy");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - docs
backup_before_write: true
destructive_requires_approval: true
exec_requires_approval: false
require_task_heartbeat: true
heartbeat_interval_ms: 30000
allowed_exec_languages:
  - python
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();

        let exec_plan = runtime.plan_exec("python script.py");
        assert!(exec_plan.allowed);
        assert!(!exec_plan.requires_approval);

        let denied_exec_plan = runtime.plan_exec("node script.js");
        assert!(!denied_exec_plan.allowed);

        let write_plan = runtime.plan_file_write(std::path::Path::new("docs/out.md"), false);
        assert!(write_plan.allowed);
        assert!(write_plan.create_backup);

        let denied_write_plan = runtime.plan_file_write(std::path::Path::new("src/out.md"), false);
        assert!(!denied_write_plan.allowed);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_can_execute_guarded_bash_command() {
        let temp_dir = create_temp_workspace("runtime-exec");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: false
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();

        let result = runtime
            .execute_exec(ExecRequest {
                program: "bash".into(),
                args: vec!["-lc".into(), "printf guarded-ok".into()],
                cwd: Some(temp_dir.clone()),
                approval_granted: false,
            })
            .unwrap();

        assert_eq!(result.stdout, "guarded-ok");
        assert_eq!(result.exit_code, Some(0));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_blocks_exec_when_approval_is_missing() {
        let temp_dir = create_temp_workspace("runtime-exec-approval");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: true
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();

        let error = runtime
            .execute_exec(ExecRequest {
                program: "bash".into(),
                args: vec!["-lc".into(), "printf denied".into()],
                cwd: Some(temp_dir.clone()),
                approval_granted: false,
            })
            .unwrap_err();

        assert!(matches!(error, RuntimeActionError::ApprovalRequired { .. }));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_writes_file_with_backup_inside_allowed_root() {
        let temp_dir = create_temp_workspace("runtime-write");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - docs
backup_before_write: true
destructive_requires_approval: false
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("docs")).unwrap();
        fs::write(temp_dir.join("docs").join("note.md"), "old").unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();

        let result = runtime
            .write_file(FileWriteRequest {
                path: temp_dir.join("docs").join("note.md"),
                content: b"new".to_vec(),
                approval_granted: false,
            })
            .unwrap();

        assert_eq!(result.bytes_written, 3);
        assert!(result.backup_path.is_some());
        assert_eq!(fs::read_to_string(temp_dir.join("docs").join("note.md")).unwrap(), "new");

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn task_exec_action_appends_tool_events() {
        let temp_dir = create_temp_workspace("task-exec-events");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: false
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Exec".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let result = runtime
            .execute_exec_for_task(
                "task-1",
                ExecRequest {
                    program: "bash".into(),
                    args: vec!["-lc".into(), "printf evented".into()],
                    cwd: Some(temp_dir.clone()),
                    approval_granted: false,
                },
                2_000,
            )
            .unwrap();

        assert_eq!(result.stdout, "evented");
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::PolicyAllowed { ref action, .. } if action == "exec"
        )));
        assert!(runtime.events().len() >= 5);
        assert_eq!(
            runtime.projection("task-1").unwrap().progress_text.as_deref(),
            Some("exec finished: bash -lc printf evented")
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn task_file_write_action_appends_tool_events() {
        let temp_dir = create_temp_workspace("task-write-events");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - docs
backup_before_write: true
destructive_requires_approval: false
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("docs")).unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Write".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let target = temp_dir.join("docs").join("task.md");
        runtime
            .write_file_for_task(
                "task-1",
                FileWriteRequest {
                    path: target.clone(),
                    content: b"hello".to_vec(),
                    approval_granted: false,
                },
                2_000,
            )
            .unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
        let expected = format!("file write finished: {}", target.display());
        assert_eq!(
            runtime.projection("task-1").unwrap().progress_text.as_deref(),
            Some(expected.as_str())
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_state_can_be_saved_and_reloaded() {
        let temp_dir = create_temp_workspace("runtime-persist");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: false
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();
        let store_dir = temp_dir.join(".taskloop");

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Persisted".into(),
                body: Some("External queue".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope.clone());
        runtime.register_task(connector.create_task_from_work_item(
            "task-1",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            1_000,
        ));
        let task = runtime.task("task-1").unwrap().clone();
        runtime.enqueue_task("task-1", 1_500, false).unwrap();
        runtime
            .schedule_task_wakeup("task-1", 8_000, "later retry", None, 2_500)
            .unwrap();
        runtime.remember(crate::memory::MemoryRecord {
            id: "memory-1".into(),
            kind: crate::memory::MemoryKind::Decision,
            scope: "workspace".into(),
            subject_ref: None,
            content: "Use TaskLoop".into(),
            source: "user".into(),
            confidence: 100,
            updated_at_ms: 2_250,
            supersedes: None,
            tags: vec!["naming".into()],
        });
        runtime
            .promote_task_to_complex_mode("task-1", 1_900, "requires planner/reviewer split")
            .unwrap();
        let started = runtime.build_task_event(&task, DomainEvent::TaskStarted, 2_000, EventVisibility::Ui);
        runtime.append_event(started);
        runtime
            .execute_exec_for_task(
                "task-1",
                ExecRequest {
                    program: "bash".into(),
                    args: vec!["-lc".into(), "printf recoverable".into()],
                    cwd: Some(temp_dir.clone()),
                    approval_granted: false,
                },
                3_000,
            )
            .unwrap();
        runtime
            .cache_artifact(
                "task-1",
                "artifact-1",
                "report.md",
                b"cached artifact",
                3_500,
                Some(10_000),
            )
            .unwrap();
        runtime
            .spawn_subagent("task-1", "sub-1", "reviewer", true, 3_600)
            .unwrap();
        runtime
            .update_subagent_status(
                "sub-1",
                SubagentStatus::Waiting,
                Some("awaiting review result".into()),
                3_700,
            )
            .unwrap();

        let store = RuntimeStateStore::new(&store_dir);
        runtime.save_state(&store).unwrap();

        let loaded = TaskLoopRuntime::load_state(&store).unwrap();
        assert_eq!(loaded.session("session-1").unwrap().title, "Demo");
        assert_eq!(loaded.channel_object("obj-1").unwrap().source_system, "feishu");
        assert_eq!(loaded.work_item("work-1").unwrap().status, "open");
        assert_eq!(loaded.task("task-1").unwrap().title, "Persisted");
        assert!(matches!(
            loaded.task("task-1").unwrap().host,
            TaskHost::External { .. }
        ));
        assert_eq!(
            loaded.projection("task-1").unwrap().workflow_mode,
            crate::state::TaskWorkflowMode::Complex
        );
        assert_eq!(
            loaded.projection("task-1").unwrap().progress_text.as_deref(),
            Some("subagent update: awaiting review result")
        );
        assert!(loaded.events().len() >= 4);
        assert_eq!(loaded.scheduler().queued().len(), 1);
        assert_eq!(loaded.scheduler().wakeups().len(), 1);
        assert_eq!(loaded.memory_store().active().len(), 1);
        assert_eq!(loaded.artifacts_for_task("task-1").len(), 1);
        assert_eq!(loaded.subagents_for_task("task-1").len(), 1);
        assert_eq!(
            loaded.subagent("sub-1").unwrap().status,
            SubagentStatus::Waiting
        );

        let continued_task = loaded.task("task-1").unwrap().clone();
        let next_event = {
            let mut mutable = loaded;
            mutable.build_task_event(
                &continued_task,
                DomainEvent::TaskHeartbeat {
                    message: Some("continued".into()),
                },
                4_000,
                EventVisibility::Ui,
            )
        };
        assert!(next_event.sequence > 1);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_can_promote_task_to_complex_mode() {
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
            title: "Complexify".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        runtime
            .promote_task_to_complex_mode("task-1", 1_500, "task needs planner and reviewer")
            .unwrap();

        assert_eq!(
            runtime.projection("task-1").unwrap().workflow_mode,
            crate::state::TaskWorkflowMode::Complex
        );
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::TaskWorkflowModeChanged { .. }
        )));
    }

    #[test]
    fn runtime_builds_task_diagnostic_report() {
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
            title: "Diagnose".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        runtime
            .promote_task_to_complex_mode("task-1", 1_100, "task requires review loop")
            .unwrap();
        let task = runtime.task("task-1").unwrap().clone();
        let blocked = runtime.build_task_event(
            &task,
            DomainEvent::TaskBlocked {
                reason: "approval queue stalled".into(),
            },
            1_200,
            EventVisibility::Ui,
        );
        runtime.append_event(blocked);
        runtime.log_warn("review queue is slow", Some("task-1".into()), 1_300);
        runtime.heartbeat_all(20_000, 5_000, 10_000);

        let report = runtime.task_diagnostic_report("task-1").unwrap();
        assert_eq!(report.task_id, "task-1");
        assert_eq!(report.workflow_mode, crate::state::TaskWorkflowMode::Complex);
        assert_eq!(report.health, crate::state::TaskHealthState::Stalled);
        assert!(report.summary.contains("workflow=Complex"));
        assert!(!report.recommendations.is_empty());
        assert!(!report.recent_logs.is_empty());
    }

    #[test]
    fn runtime_tracks_subagents_under_parent_task() {
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
            title: "Parent".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        runtime
            .spawn_subagent("task-1", "sub-1", "reviewer", false, 1_100)
            .unwrap();
        runtime
            .update_subagent_status(
                "sub-1",
                SubagentStatus::Running,
                Some("reviewing draft".into()),
                1_200,
            )
            .unwrap();

        assert_eq!(runtime.subagents_for_task("task-1").len(), 1);
        assert_eq!(runtime.subagent("sub-1").unwrap().parent_task_id, "task-1");
        assert_eq!(
            runtime.projection("task-1").unwrap().waiting_on,
            Some(crate::state::TaskWaitKind::Subtask)
        );
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::SubagentSpawned { .. }
        )));
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::SubagentUpdated { .. }
        )));
    }

    #[test]
    fn runtime_builds_foreground_and_background_task_board_entries() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_task(Task {
            id: "task-fg".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Foreground".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        runtime.register_task(Task {
            id: "task-bg".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Background".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_100,
        });

        runtime.enqueue_task("task-fg", 1_200, false).unwrap();
        runtime.enqueue_task("task-bg", 1_300, true).unwrap();

        let entries = runtime.task_board_entries();
        let foreground = entries.iter().find(|entry| entry.task_id == "task-fg").unwrap();
        let background = entries.iter().find(|entry| entry.task_id == "task-bg").unwrap();

        assert_eq!(foreground.attention_lane, TaskAttentionLane::Foreground);
        assert_eq!(background.attention_lane, TaskAttentionLane::Background);
        assert_eq!(foreground.host_label, "agentboard");
        assert!(foreground.is_queued);
        assert!(background.is_queued);
    }

    #[test]
    fn runtime_builds_task_board_snapshot_with_host_counts() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_task(Task {
            id: "task-local".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Local task".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        runtime.register_task(Task {
            id: "task-feishu".into(),
            session_id: "session-1".into(),
            work_item_id: Some("work-1".into()),
            title: "Feishu task".into(),
            kind: TaskKind::ExternalHost,
            host: TaskHost::External {
                system: "feishu".into(),
                object_type: "task".into(),
                object_id: "ext-1".into(),
            },
            priority: TaskPriority::High,
            created_at_ms: 1_100,
        });

        runtime.enqueue_task("task-local", 1_200, false).unwrap();
        runtime.enqueue_task("task-feishu", 1_300, true).unwrap();
        runtime
            .promote_task_to_complex_mode("task-feishu", 1_350, "external board workflow")
            .unwrap();
        runtime.heartbeat_all(20_000, 5_000, 10_000);

        let snapshot = runtime.task_board_snapshot(20_000);

        assert_eq!(snapshot.generated_at_ms, 20_000);
        assert_eq!(snapshot.summary.total_tasks, 2);
        assert_eq!(snapshot.summary.foreground_tasks, 1);
        assert_eq!(snapshot.summary.background_tasks, 1);
        assert_eq!(snapshot.summary.queued_tasks, 2);
        assert_eq!(snapshot.summary.complex_tasks, 1);
        assert_eq!(snapshot.summary.stalled_tasks, 2);
        assert!(snapshot
            .host_counts
            .iter()
            .any(|bucket| bucket.host_label == "agentboard" && bucket.task_count == 1));
        assert!(snapshot
            .host_counts
            .iter()
            .any(|bucket| bucket.host_label == "feishu" && bucket.task_count == 1));
    }

    #[test]
    fn runtime_builds_client_surface_snapshot() {
        let mut runtime = TaskLoopRuntime::new();
        let temp_dir = create_temp_workspace("runtime-client-surface");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("config")).unwrap();
        fs::write(
            temp_dir.join("config").join("providers.toml"),
            r#"
[default]
provider_id = "openai_default"
base_url = "https://api.example.com/v1"
protocol = "openai-compatible"
api_family = "chat-completions"
auth = "bearer"
header_policy = "strict"
user_agent_mode = "runtime-default"
models = ["gpt-4.1", "gpt-4.1-mini"]
capabilities = ["streaming", "tool-calling", "json-output"]
"#,
        )
        .unwrap();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Main".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_session(Session {
            id: "session-2".into(),
            title: "Ops".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        runtime.register_task(Task {
            id: "task-local".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Local task".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        runtime.register_task(Task {
            id: "task-feishu".into(),
            session_id: "session-2".into(),
            work_item_id: Some("work-1".into()),
            title: "Feishu task".into(),
            kind: TaskKind::ExternalHost,
            host: TaskHost::External {
                system: "feishu".into(),
                object_type: "task".into(),
                object_id: "ext-1".into(),
            },
            priority: TaskPriority::High,
            created_at_ms: 1_100,
        });

        runtime.enqueue_task("task-local", 1_200, false).unwrap();
        runtime.enqueue_task("task-feishu", 1_300, true).unwrap();
        let local_task = runtime.task("task-local").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &local_task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_350,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &local_task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:0:exec".into()),
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let snapshot = runtime.client_surface_snapshot(2_000);

        assert_eq!(snapshot.total_sessions, 2);
        assert_eq!(snapshot.total_channel_objects, 1);
        assert_eq!(snapshot.total_work_items, 1);
        assert_eq!(snapshot.task_board.summary.total_tasks, 2);
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.work_items.len(), 1);
        assert_eq!(snapshot.approval_queue.len(), 1);
        assert_eq!(snapshot.approval_summary.pending_total, 1);
        assert_eq!(snapshot.approval_summary.pending_high_risk, 1);
        assert!(snapshot
            .approval_view_presets
            .iter()
            .any(|view| view.view_key == "pending_high_risk" && view.item_count == 1));
        assert!(snapshot
            .approval_view_presets
            .iter()
            .any(|view| view.view_key == "pending_all" && view.item_count == 1));
        assert_eq!(snapshot.channel_hosts.len(), 1);
        assert_eq!(snapshot.channel_inbox.len(), 1);
        assert_eq!(snapshot.sessions.len(), 2);
        assert!(snapshot
            .sessions
            .iter()
            .any(|entry| entry.session_id == "session-1" && entry.total_tasks == 1));
        assert!(snapshot
            .sessions
            .iter()
            .any(|entry| entry.session_id == "session-2" && entry.total_tasks == 1));
        assert!(snapshot
            .channel_sources
            .iter()
            .any(|bucket| bucket.source_system == "feishu" && bucket.object_count == 1));
        assert!(snapshot
            .providers
            .iter()
            .any(|provider| provider.profile == "default"
                && provider.protocol_label == "openai-compatible"
                && provider.api_family_label == "chat-completions"
                && provider.header_policy_label == "strict"
                && provider.model_count == 2
                && provider
                    .capability_labels
                    .iter()
                    .any(|value| value == "tool-calling")));
        assert!(snapshot
            .waiting_buckets
            .iter()
            .any(|bucket| bucket.wait_kind_label == "approval" && bucket.task_count == 1));
        assert!(snapshot.work_items.iter().any(|item| {
            item.work_item_id == "work-1"
                && item.source_system == "feishu"
                && item.status == "open"
                && item.channel_object_id.as_deref() == Some("obj-1")
        }));
        assert!(snapshot.approval_queue.iter().any(|item| {
            item.approval_id == "approval:task-local:exec:action:0:exec"
                && item.task_id == "task-local"
                && item.session_id == "session-1"
                && item.host_label == "agentboard"
                && item.approval_action.as_deref() == Some("exec")
                && item.approval_risk_label.as_deref() == Some("high")
                && item.approval_status == "pending"
                && item.requested_at_ms == Some(1_400)
                && item.sort_key.starts_with("0:")
                && item.resume_checkpoint.as_deref() == Some("action:0:exec")
                && item.reason.as_deref() == Some("needs confirmation")
                && item.primary_action == "open_approval"
                && item.secondary_actions == vec!["open_task", "inspect_policy"]
        }));
        assert!(snapshot
            .approval_groups
            .iter()
            .any(|bucket| bucket.bucket_key == "pending:exec:high" && bucket.item_count == 1));
        assert!(snapshot.channel_inbox.iter().any(|item| {
            item.work_item_id == "work-1"
                && item.source_system == "feishu"
                && item.object_type == "task"
                && item.host_kind == "task_list"
                && item.host_display_name == "Feishu Tasks"
                && item.sync_mode_label == "bidirectional"
                && item.capability_labels
                    == vec!["task_creation", "status_sync", "comment_sync", "attachment_export"]
                && item.requires_bidirectional_binding
                && item.external_id == "ext-1"
                && item.workspace_id.as_deref() == Some("space-1")
                && item.linked_task_id.as_deref() == Some("task-feishu")
                && item.linked_task_lifecycle.as_deref() == Some("queued")
                && item.linked_task_waiting_on.is_none()
                && !item.needs_attention
                && item.supports_writeback
                && item.supports_background_sync
                && item.primary_action == "open_work_item"
                && item.secondary_actions
                    == vec!["open_linked_task", "open_channel_object", "sync_external"]
        }));
        assert!(snapshot.channel_hosts.iter().any(|host| {
            host.host_key == "feishu:space-1:task"
                &&
            host.source_system == "feishu"
                && host.workspace_id.as_deref() == Some("space-1")
                && host.object_type == "task"
                && host.host_kind == "task_list"
                && host.host_display_name == "Feishu Tasks"
                && host.sync_mode_label == "bidirectional"
                && host.capability_labels
                    == vec!["task_creation", "status_sync", "comment_sync", "attachment_export"]
                && host.requires_bidirectional_binding
                && host.display_label == "feishu:space-1:task"
                && host.item_count == 1
                && host.attention_count == 0
                && host.open_count == 1
                && host.supports_writeback
                && host.supports_background_sync
                && host.primary_action == "open_host_list"
                && host.secondary_actions == vec!["sync_host", "open_host_settings"]
        }));
    }

    #[test]
    fn connector_envelope_maps_external_object_to_internal_models() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);

        let task = connector.create_task_from_work_item(
            "task-1",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            1_000,
        );
        runtime.register_task(task);

        assert_eq!(runtime.channel_object("obj-1").unwrap().object_type, "task");
        assert_eq!(
            runtime.work_item("work-1").unwrap().channel_object_id.as_deref(),
            Some("obj-1")
        );
        assert_eq!(runtime.task("task-1").unwrap().work_item_id.as_deref(), Some("work-1"));
        assert!(matches!(
            runtime.task("task-1").unwrap().host,
            TaskHost::External {
                ref system,
                ref object_type,
                ref object_id,
            } if system == "feishu" && object_type == "task" && object_id == "ext-1"
        ));
    }

    #[test]
    fn scheduler_resumes_due_background_tasks() {
        let temp_dir = create_temp_workspace("runtime-scheduler");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Wakeup".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::High,
            created_at_ms: 1_000,
        });

        runtime
            .schedule_task_wakeup("task-1", 5_000, "background retry", None, 2_000)
            .unwrap();
        let resumed = runtime.process_due_wakeups(5_000).unwrap();
        assert_eq!(resumed, vec!["task-1"]);
        assert_eq!(runtime.scheduler().queued().len(), 1);
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Running
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_resumes_task_from_matching_checkpoint() {
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
            title: "Resume me".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let waiting = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:2:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting);

        runtime
            .resume_task_from_checkpoint("task-1", "action:2:exec", 2_000, false)
            .unwrap();

        assert_eq!(runtime.scheduler().queued().len(), 1);
        assert_eq!(
            runtime.scheduler().queued()[0].task_id,
            "task-1"
        );
        assert_eq!(
            runtime.projection("task-1").unwrap().resume_checkpoint,
            None
        );
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Running
        );
    }

    #[test]
    fn runtime_rejects_mismatched_resume_checkpoint() {
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
            title: "Resume me".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let waiting = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:2:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting);

        let error = runtime
            .resume_task_from_checkpoint("task-1", "action:9:exec", 2_000, false)
            .unwrap_err();
        assert!(matches!(
            error,
            RuntimeActionError::InvalidResumeCheckpoint { .. }
        ));
    }

    #[test]
    fn runtime_approves_task_checkpoint_via_explicit_method() {
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
            title: "Approve me".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let waiting = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:3:write_file".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting);

        runtime
            .approve_task_checkpoint("task-1", "action:3:write_file", 2_000, false)
            .unwrap();

        assert_eq!(runtime.scheduler().queued().len(), 1);
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Running
        );
    }

    #[test]
    fn runtime_rejects_task_checkpoint_via_explicit_method() {
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
            title: "Reject me".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let waiting = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting);

        runtime
            .reject_task_checkpoint("task-1", "action:1:exec", "user rejected", 2_000)
            .unwrap();

        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Cancelled
        );
        assert_eq!(
            runtime.projection("task-1").unwrap().blocked_reason.as_deref(),
            Some("approval rejected: user rejected")
        );
        assert_eq!(runtime.projection("task-1").unwrap().resume_checkpoint, None);
    }

    #[test]
    fn runtime_approves_pending_approval_by_id() {
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
            title: "Approve by id".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        runtime
            .approve_pending_approval("approval:task-1:exec:action:1:exec", 2_000, false)
            .unwrap();
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Running
        );
    }

    #[test]
    fn runtime_rejects_pending_approval_by_id() {
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
            title: "Reject by id".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        runtime
            .reject_pending_approval(
                "approval:task-1:exec:action:1:exec",
                "user rejected",
                2_000,
            )
            .unwrap();
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Cancelled
        );
    }

    #[test]
    fn runtime_batch_approves_pending_approvals() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        for task_id in ["task-1", "task-2"] {
            runtime.register_task(Task {
                id: task_id.into(),
                session_id: "session-1".into(),
                work_item_id: None,
                title: format!("Task {task_id}"),
                kind: TaskKind::Internal,
                host: TaskHost::Internal,
                priority: TaskPriority::Normal,
                created_at_ms: 1_000,
            });
            let task = runtime.task(task_id).unwrap().clone();
            let approval_event = runtime.build_task_event(
                &task,
                DomainEvent::PolicyApprovalRequired {
                    action: "exec".into(),
                    reason: "needs confirmation".into(),
                    risk_level: crate::policy::ApprovalRiskLevel::High,
                },
                1_400,
                EventVisibility::Ui,
            );
            runtime.append_event(approval_event);
            let waiting_event = runtime.build_task_event(
                &task,
                DomainEvent::TaskWaiting {
                    kind: crate::state::TaskWaitKind::Approval,
                    reason: "needs confirmation".into(),
                    resume_checkpoint: Some("action:1:exec".into()),
                },
                1_500,
                EventVisibility::Ui,
            );
            runtime.append_event(waiting_event);
        }

        let approvals = vec![
            "approval:task-1:exec:action:1:exec".to_string(),
            "approval:task-2:exec:action:1:exec".to_string(),
        ];
        let result = runtime.approve_pending_approvals(&approvals, 2_000, false);

        assert_eq!(result.requested, 2);
        assert_eq!(result.succeeded.len(), 2);
        assert!(result.failed.is_empty());
        assert_eq!(runtime.scheduler().queued().len(), 2);
    }

    #[test]
    fn runtime_builds_grouped_client_api_snapshot() {
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
            title: "Grouped API".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let snapshot = runtime.client_api_snapshot(2_000);
        assert_eq!(snapshot.generated_at_ms, 2_000);
        assert_eq!(snapshot.total_sessions, 1);
        assert_eq!(snapshot.task_center.task_board.summary.total_tasks, 1);
        assert_eq!(snapshot.task_center.summary.total_tasks, 1);
        assert!(snapshot
            .task_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "open_foreground_tasks" && action.enabled));
        assert!(snapshot
            .task_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "cancel_selected" && action.reason_required));
        assert!(snapshot
            .task_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "requeue_selected" && action.bulk));
        assert!(snapshot
            .task_center
            .view_presets
            .iter()
            .any(|view| view.view_key == "all_tasks" && view.item_count == 1));
        assert_eq!(snapshot.approval_center.summary.pending_total, 1);
        assert_eq!(snapshot.approval_center.queue.len(), 1);
        assert!(snapshot
            .approval_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "approve_selected" && action.enabled));
        assert!(snapshot
            .approval_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "reject_selected" && action.reason_required));
        assert!(snapshot
            .connector_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "open_channel_hosts" && !action.enabled));
        assert!(snapshot
            .connector_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "sync_selected_hosts" && !action.enabled));
        assert_eq!(snapshot.connector_center.summary.total_hosts, 0);
        assert_eq!(snapshot.connector_center.summary.syncable_hosts, 0);
        assert_eq!(snapshot.connector_center.summary.bidirectional_hosts, 0);
        assert!(snapshot
            .connector_center
            .view_presets
            .iter()
            .any(|view| view.view_key == "all_hosts" && view.item_count == 0));
        assert!(snapshot
            .connector_center
            .view_presets
            .iter()
            .any(|view| view.view_key == "syncable_hosts" && view.item_count == 0));
        assert_eq!(snapshot.connector_center.providers.len(), 0);
    }

    #[test]
    fn runtime_builds_connector_center_sync_views() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        let task = connector.create_task_from_work_item(
            "task-1",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            1_000,
        );
        runtime.register_task(task);

        let snapshot = runtime.client_api_snapshot(2_000);
        assert_eq!(snapshot.connector_center.summary.total_hosts, 1);
        assert_eq!(snapshot.connector_center.summary.syncable_hosts, 1);
        assert_eq!(snapshot.connector_center.summary.bidirectional_hosts, 1);
        assert!(snapshot
            .connector_center
            .available_actions
            .iter()
            .any(|action| action.action_key == "sync_selected_hosts" && action.enabled));
        assert!(snapshot
            .connector_center
            .view_presets
            .iter()
            .any(|view| view.view_key == "syncable_hosts" && view.item_count == 1));
        assert!(snapshot
            .connector_center
            .view_presets
            .iter()
            .any(|view| view.view_key == "bidirectional_hosts" && view.item_count == 1));
    }

    #[test]
    fn runtime_syncs_channel_hosts_and_reports_batch_results() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        let task = connector.create_task_from_work_item(
            "task-1",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            1_000,
        );
        runtime.register_task(task);

        let result = runtime.sync_channel_hosts(
            &[("feishu".into(), Some("space-1".into()), "task".into())],
            2_000,
        );
        assert_eq!(result.requested, 1);
        assert_eq!(result.succeeded, vec!["feishu:space-1:task"]);
        assert!(result.failed.is_empty());
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::ExternalSynced { ref connector_id } if connector_id == "feishu:task"
        )));
    }

    #[test]
    fn runtime_syncs_channel_hosts_by_key() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        let task = connector.create_task_from_work_item(
            "task-1",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            1_000,
        );
        runtime.register_task(task);

        let result = runtime.sync_channel_host_keys(&["feishu:space-1:task".into()], 2_000);
        assert_eq!(result.requested, 1);
        assert_eq!(result.succeeded, vec!["feishu:space-1:task"]);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn runtime_executes_center_actions_through_unified_dispatchers() {
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

        let task_action_result = runtime
            .execute_task_center_action(
                "cancel_selected",
                &["task-1".into()],
                Some("user cancelled".into()),
                2_000,
                false,
            )
            .unwrap();
        assert_eq!(task_action_result.action_key, "cancel_selected");
        assert_eq!(task_action_result.succeeded, vec!["task-1"]);

        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            2_100,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            2_200,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let approval_action_result = runtime
            .execute_approval_center_action(
                "approve_selected",
                &["approval:task-1:exec:action:1:exec".into()],
                None,
                2_300,
                false,
            )
            .unwrap();
        assert_eq!(approval_action_result.action_key, "approve_selected");
        assert_eq!(
            approval_action_result.succeeded,
            vec!["approval:task-1:exec:action:1:exec"]
        );

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        let ext_task = connector.create_task_from_work_item(
            "task-ext",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            3_000,
        );
        runtime.register_task(ext_task);

        let connector_action_result = runtime
            .execute_connector_center_action(
                "sync_selected_hosts",
                &["feishu:space-1:task".into()],
                3_500,
            )
            .unwrap();
        assert_eq!(connector_action_result.action_key, "sync_selected_hosts");
        assert_eq!(
            connector_action_result.succeeded,
            vec!["feishu:space-1:task"]
        );
    }

    #[test]
    fn runtime_executes_unified_client_action_requests() {
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

        let task_result = runtime
            .execute_client_action(crate::client_api::ClientActionRequest {
                center: crate::client_api::ClientCenterKind::Task,
                action_key: "cancel_selected".into(),
                item_keys: vec!["task-1".into()],
                reason: Some("user cancelled".into()),
                timestamp_ms: 2_000,
                background: false,
            })
            .unwrap();
        assert_eq!(task_result.action_key, "cancel_selected");

        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            2_100,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            2_200,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let approval_result = runtime
            .execute_client_action(crate::client_api::ClientActionRequest {
                center: crate::client_api::ClientCenterKind::Approval,
                action_key: "approve_selected".into(),
                item_keys: vec!["approval:task-1:exec:action:1:exec".into()],
                reason: None,
                timestamp_ms: 2_300,
                background: false,
            })
            .unwrap();
        assert_eq!(approval_result.action_key, "approve_selected");

        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );
        runtime.ingest_connector_envelope(envelope);
        let ext_task = connector.create_task_from_work_item(
            "task-ext",
            "session-1",
            runtime.work_item("work-1").unwrap(),
            runtime.channel_object("obj-1").unwrap(),
            3_000,
        );
        runtime.register_task(ext_task);

        let connector_result = runtime
            .execute_client_action(crate::client_api::ClientActionRequest {
                center: crate::client_api::ClientCenterKind::Connector,
                action_key: "sync_selected_hosts".into(),
                item_keys: vec!["feishu:space-1:task".into()],
                reason: None,
                timestamp_ms: 3_500,
                background: false,
            })
            .unwrap();
        assert_eq!(connector_result.action_key, "sync_selected_hosts");
    }

    #[test]
    fn runtime_batch_reject_collects_failures() {
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
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        let approvals = vec![
            "approval:task-1:exec:action:1:exec".to_string(),
            "approval:missing:exec:action:1:exec".to_string(),
        ];
        let result = runtime.reject_pending_approvals(&approvals, "user rejected", 2_000);

        assert_eq!(result.requested, 2);
        assert_eq!(result.succeeded, vec!["approval:task-1:exec:action:1:exec"]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(runtime.projection("task-1").unwrap().lifecycle, TaskLifecycleState::Cancelled);
    }

    #[test]
    fn runtime_cancels_and_requeues_tasks_in_batch() {
        let mut runtime = TaskLoopRuntime::new();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: "/tmp/agentboard".into(),
        });
        for task_id in ["task-1", "task-2"] {
            runtime.register_task(Task {
                id: task_id.into(),
                session_id: "session-1".into(),
                work_item_id: None,
                title: format!("Task {task_id}"),
                kind: TaskKind::Internal,
                host: TaskHost::Internal,
                priority: TaskPriority::Normal,
                created_at_ms: 1_000,
            });
            runtime.enqueue_task(task_id, 1_500, false).unwrap();
        }

        let cancel_ids = vec!["task-1".to_string()];
        let cancel_result = runtime.cancel_tasks(&cancel_ids, "user cancelled", 2_000);
        assert_eq!(cancel_result.requested, 1);
        assert_eq!(cancel_result.succeeded, vec!["task-1"]);
        assert_eq!(runtime.projection("task-1").unwrap().lifecycle, TaskLifecycleState::Cancelled);

        let requeue_ids = vec!["task-1".to_string(), "task-2".to_string()];
        let requeue_result = runtime.requeue_tasks(&requeue_ids, 3_000, false);
        assert_eq!(requeue_result.requested, 2);
        assert_eq!(requeue_result.succeeded.len(), 2);
        assert!(requeue_result.failed.is_empty());
        assert_eq!(runtime.scheduler().queued().len(), 2);

        let client_result = crate::client_api::map_task_batch_result(
            "requeue_selected",
            requeue_result,
        );
        assert_eq!(client_result.action_key, "requeue_selected");
        assert_eq!(client_result.requested, 2);
        assert_eq!(client_result.succeeded.len(), 2);
    }

    #[test]
    fn client_surface_snapshot_includes_approval_history() {
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
            title: "Approval history".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        let task = runtime.task("task-1").unwrap().clone();
        let approval_event = runtime.build_task_event(
            &task,
            DomainEvent::PolicyApprovalRequired {
                action: "exec".into(),
                reason: "needs confirmation".into(),
                risk_level: crate::policy::ApprovalRiskLevel::High,
            },
            1_400,
            EventVisibility::Ui,
        );
        runtime.append_event(approval_event);
        let waiting_event = runtime.build_task_event(
            &task,
            DomainEvent::TaskWaiting {
                kind: crate::state::TaskWaitKind::Approval,
                reason: "needs confirmation".into(),
                resume_checkpoint: Some("action:1:exec".into()),
            },
            1_500,
            EventVisibility::Ui,
        );
        runtime.append_event(waiting_event);

        runtime
            .approve_pending_approval("approval:task-1:exec:action:1:exec", 2_000, false)
            .unwrap();
        let snapshot = runtime.client_surface_snapshot(2_100);
        assert_eq!(snapshot.approval_summary.approved_total, 1);
        assert_eq!(snapshot.approval_summary.rejected_total, 0);
        assert_eq!(snapshot.approval_history_window.total_items, 1);
        assert_eq!(snapshot.approval_history_window.visible_items, 1);
        assert_eq!(
            snapshot.approval_history_window.limit,
            crate::presentation::DEFAULT_APPROVAL_HISTORY_LIMIT as u32
        );
        assert!(snapshot
            .approval_view_presets
            .iter()
            .any(|view| view.view_key == "approved_recent" && view.item_count == 1));
        assert!(snapshot
            .approval_view_presets
            .iter()
            .any(|view| view.view_key == "rejected_recent" && view.item_count == 0));
        assert!(snapshot.approval_history.iter().any(|item| {
            item.approval_id == "approval:task-1:exec:action:1:exec"
                && item.status == "approved"
                && item.approval_action == "exec"
                && item.approval_risk_label == "high"
                && item.decision_source == "user"
                && item.resolved_by == "user"
                && item.resolved_at_ms == 2_000
        }));
        assert!(snapshot
            .approval_groups
            .iter()
            .any(|bucket| bucket.bucket_key == "approved:exec:high" && bucket.item_count == 1));
    }

    #[test]
    fn scripted_single_agent_loop_runs_task_to_completion() {
        let temp_dir = create_temp_workspace("runtime-scripted-loop");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - docs
exec_requires_approval: false
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();
        fs::create_dir_all(temp_dir.join("docs")).unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Loop".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });
        runtime.enqueue_task("task-1", 1_200, false).unwrap();

        let report = runtime
            .process_next_queued_script(
                TaskScript::new(vec![
                    TaskAction::Progress {
                        phase: Some("reason".into()),
                        current_step: Some("1".into()),
                        total_steps: Some(3),
                        message: "planning".into(),
                    },
                    TaskAction::Exec {
                        request: ExecRequest {
                            program: "bash".into(),
                            args: vec!["-lc".into(), "printf loop-ok".into()],
                            cwd: Some(temp_dir.clone()),
                            approval_granted: false,
                        },
                    },
                    TaskAction::WriteFile {
                        request: FileWriteRequest {
                            path: temp_dir.join("docs").join("loop.md"),
                            content: b"done".to_vec(),
                            approval_granted: false,
                        },
                    },
                    TaskAction::Remember {
                        record: crate::memory::MemoryRecord {
                            id: "memory-1".into(),
                            kind: crate::memory::MemoryKind::TaskState,
                            scope: "task".into(),
                            subject_ref: Some("task-1".into()),
                            content: "loop completed".into(),
                            source: "runtime".into(),
                            confidence: 100,
                            updated_at_ms: 2_500,
                            supersedes: None,
                            tags: vec!["task".into()],
                        },
                    },
                ]),
                2_000,
            )
            .unwrap()
            .unwrap();

        assert!(report.completed);
        assert_eq!(fs::read_to_string(temp_dir.join("docs").join("loop.md")).unwrap(), "done");
        assert_eq!(runtime.memory_store().active().len(), 1);
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Done
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn scripted_loop_can_leave_task_waiting_for_wakeup() {
        let temp_dir = create_temp_workspace("runtime-scripted-wakeup");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Wake".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let report = runtime
            .run_task_script(
                "task-1",
                TaskScript::new(vec![TaskAction::ScheduleWakeup {
                    wake_at_ms: 9_000,
                    reason: "background continuation".into(),
                }]),
                2_000,
            )
            .unwrap();

        assert!(report.waiting_for_wakeup);
        assert!(!report.completed);
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Waiting
        );
        assert_eq!(runtime.scheduler().wakeups().len(), 1);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_writes_memory_markdown_projections() {
        let temp_dir = create_temp_workspace("runtime-memory-projection");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - .
backup_before_write: true
destructive_requires_approval: false
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.remember(crate::memory::MemoryRecord {
            id: "memory-1".into(),
            kind: crate::memory::MemoryKind::Decision,
            scope: "workspace".into(),
            subject_ref: Some("naming".into()),
            content: "AgentBoard is the product name".into(),
            source: "user".into(),
            confidence: 100,
            updated_at_ms: 2_000,
            supersedes: None,
            tags: vec!["branding".into()],
        });
        runtime.remember(crate::memory::MemoryRecord {
            id: "memory-2".into(),
            kind: crate::memory::MemoryKind::TaskState,
            scope: "task".into(),
            subject_ref: Some("task-1".into()),
            content: "Task is running".into(),
            source: "runtime".into(),
            confidence: 90,
            updated_at_ms: 2_500,
            supersedes: None,
            tags: vec!["status".into()],
        });

        runtime.write_workspace_memory_projection("2026-03-25").unwrap();
        runtime.write_daily_memory_projection("2026-03-25").unwrap();

        let stable = fs::read_to_string(temp_dir.join("MEMORY.md")).unwrap();
        let daily = fs::read_to_string(temp_dir.join("memory").join("daily").join("2026-03-25.md"))
            .unwrap();

        assert!(stable.contains("# Stable Workspace Facts"));
        assert!(stable.contains("AgentBoard is the product name"));
        assert!(!stable.contains("Task is running"));
        assert!(daily.contains("# Daily Memory Projection"));
        assert!(daily.contains("Task is running"));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_caches_artifacts_and_cleans_up_expired_entries() {
        let temp_dir = create_temp_workspace("runtime-artifact-cache");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Artifact".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let artifact = runtime
            .cache_artifact(
                "task-1",
                "artifact-1",
                "report.md",
                b"cached artifact",
                1_000,
                Some(10),
            )
            .unwrap();
        assert!(std::path::Path::new(&artifact.path).exists());
        assert_eq!(runtime.artifacts_for_task("task-1").len(), 1);
        let export_path = temp_dir.join("exports").join("report-copy.md");
        let exported = runtime.export_artifact_to("artifact-1", &export_path).unwrap();
        assert_eq!(exported, export_path);
        assert_eq!(fs::read_to_string(exported).unwrap(), "cached artifact");

        let removed = runtime.cleanup_expired_artifact_cache(1_020).unwrap();
        assert_eq!(removed, vec!["artifact-1"]);
        assert!(runtime.artifact("artifact-1").is_none());
        assert!(!std::path::Path::new(&artifact.path).exists());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn runtime_persists_log_levels_across_reload() {
        let temp_dir = create_temp_workspace("runtime-logs");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
---

Rules
"#,
        )
        .unwrap();

        let store_dir = temp_dir.join(".taskloop");
        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.log_debug("boot", 1_000);
        runtime.log_info("ready", 1_100);
        runtime.log_warn("slow task", Some("task-1".into()), 1_200);
        runtime.log_error("tool failed", Some("task-1".into()), 1_300);
        runtime.save_state(&RuntimeStateStore::new(&store_dir)).unwrap();

        let loaded = TaskLoopRuntime::load_state(&RuntimeStateStore::new(&store_dir)).unwrap();
        assert_eq!(loaded.logs().len(), 4);
        assert!(loaded.logs().iter().any(|log| matches!(
            log.level,
            crate::runtime_log::RuntimeLogLevel::Debug
        )));
        assert!(loaded.logs().iter().any(|log| matches!(
            log.level,
            crate::runtime_log::RuntimeLogLevel::Error
        )));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn scripted_loop_waits_for_approval_without_failing_task() {
        let temp_dir = create_temp_workspace("runtime-scripted-approval");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: true
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Needs approval".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let report = runtime
            .run_task_script(
                "task-1",
                TaskScript::new(vec![TaskAction::Exec {
                    request: ExecRequest {
                        program: "bash".into(),
                        args: vec!["-lc".into(), "printf gated".into()],
                        cwd: Some(temp_dir.clone()),
                        approval_granted: false,
                    },
                }]),
                2_000,
            )
            .unwrap();

        assert!(report.waiting_for_approval);
        assert!(!report.completed);
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::PolicyApprovalRequired { ref action, .. } if action == "exec"
        )));
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Waiting
        );
        assert_eq!(
            runtime.projection("task-1").unwrap().waiting_on,
            Some(crate::state::TaskWaitKind::Approval)
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn scripted_loop_can_replan_after_policy_suggests_safer_path() {
        let temp_dir = create_temp_workspace("runtime-scripted-replan");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: false
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Replan".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        });

        let report = runtime
            .run_task_script(
                "task-1",
                TaskScript::new(vec![
                    TaskAction::Exec {
                        request: ExecRequest {
                            program: "ruby".into(),
                            args: vec!["script.rb".into()],
                            cwd: Some(temp_dir.clone()),
                            approval_granted: false,
                        },
                    },
                    TaskAction::Heartbeat {
                        message: Some("fallback path running".into()),
                    },
                    TaskAction::Complete { summary: None },
                ]),
                2_000,
            )
            .unwrap();

        assert_eq!(report.replanned_actions, 1);
        assert!(report.completed);
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::PolicyDenied { ref action, .. } if action == "exec"
        )));
        assert!(runtime.events().iter().any(|event| matches!(
            event.event,
            DomainEvent::TaskProgress { ref phase, ref progress_text, .. }
            if phase.as_deref() == Some("replan") && progress_text.contains("replan suggested")
        )));
        assert_eq!(
            runtime.projection("task-1").unwrap().lifecycle,
            TaskLifecycleState::Done
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn queued_tasks_can_interleave_while_one_waits_for_approval() {
        let temp_dir = create_temp_workspace("runtime-approval-interleave");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
exec_requires_approval: true
allowed_exec_languages:
  - bash
---

Rules
"#,
        )
        .unwrap();

        let mut runtime = TaskLoopRuntime::new();
        runtime
            .load_workspace_contracts(&WorkspaceLayout::new(&temp_dir))
            .unwrap();
        runtime.register_session(Session {
            id: "session-1".into(),
            title: "Demo".into(),
            workspace_root: temp_dir.to_string_lossy().to_string(),
        });
        runtime.register_task(Task {
            id: "task-approval".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Approval".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::High,
            created_at_ms: 1_000,
        });
        runtime.register_task(Task {
            id: "task-2".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Follow-on".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_100,
        });
        runtime.enqueue_task("task-approval", 1_500, false).unwrap();
        runtime.enqueue_task("task-2", 1_600, false).unwrap();

        let first = runtime
            .process_next_queued_script(
                TaskScript::new(vec![TaskAction::Exec {
                    request: ExecRequest {
                        program: "bash".into(),
                        args: vec!["-lc".into(), "printf gated".into()],
                        cwd: Some(temp_dir.clone()),
                        approval_granted: false,
                    },
                }]),
                2_000,
            )
            .unwrap()
            .unwrap();
        assert!(first.waiting_for_approval);
        assert_eq!(runtime.scheduler().queued().len(), 1);

        let second = runtime
            .process_next_queued_script(
                TaskScript::new(vec![TaskAction::Heartbeat {
                    message: Some("second task continued".into()),
                }]),
                3_000,
            )
            .unwrap()
            .unwrap();
        assert!(!second.waiting_for_approval);
        assert!(second.completed);
        assert_eq!(runtime.scheduler().queued().len(), 0);
        assert_eq!(
            runtime.projection("task-approval").unwrap().waiting_on,
            Some(crate::state::TaskWaitKind::Approval)
        );
        assert_eq!(
            runtime.projection("task-2").unwrap().lifecycle,
            TaskLifecycleState::Done
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    fn create_temp_workspace(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agentboard-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
