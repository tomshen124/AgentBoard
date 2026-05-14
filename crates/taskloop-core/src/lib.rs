pub mod artifact;
pub mod bridge;
pub mod client_api;
pub mod client_demo;
pub mod client_flow;
pub mod client_runtime_api;
pub mod client_shell;
pub mod client_view;
pub mod contracts;
pub mod connector;
pub mod context;
pub mod diagnostics;
pub mod event;
pub mod execution;
pub mod loop_runner;
pub mod memory;
pub mod model;
pub mod policy;
pub mod persistence;
pub mod presentation;
pub mod provider;
pub mod registry;
pub mod runtime;
pub mod runtime_log;
pub mod scheduler;
pub mod skill_gating;
pub mod state;

pub use contracts::{
    ApiFamily, AuthMode, ConnectorSyncMode, ContractLoadError, DiscoveredSkill, HeaderPolicy,
    MarkdownContract, MarkdownContractKind, MarkdownFrontmatter, ModelAdapter, ProviderCapability,
    ProviderProtocol, SkillMetadata, SkillMode, UserAgentMode, WorkspaceContracts, WorkspaceLayout,
};
pub use connector::{
    ChannelHostKind, ChannelHostMetadata, ChannelObjectPayload, ConnectorEnvelope,
    ConnectorSample, WorkItemInput,
};
pub use context::{assemble_prompt_context, PromptContextBundle, PromptContextSection};
pub use diagnostics::{build_task_diagnostic_report, TaskDiagnosticReport};
pub use event::{DomainEvent, EventEnvelope, EventObjectRef, EventSource, EventVisibility};
pub use execution::{
    ExecRequest, ExecResult, FileWriteRequest, FileWriteResult, RuntimeActionError,
};
pub use artifact::{default_artifact_cache_root, DEFAULT_ARTIFACT_TTL_MS};
pub use client_api::{
    build_approval_center_actions, build_client_api_snapshot,
    build_connector_center_actions, build_connector_center_summary,
    build_connector_center_view_presets, build_task_center_actions, build_task_center_summary,
    build_task_center_view_presets, map_approval_batch_result, map_connector_batch_result,
    map_task_batch_result, ApprovalCenterSnapshot as ClientApprovalCenterSnapshot,
    ClientActionDescriptor, ClientActionFailure, ClientActionRequest, ClientActionResult,
    ClientApiSnapshot, ClientCenterKind, ConnectorCenterSnapshot, ConnectorCenterSummary,
    CenterViewPreset, TaskCenterSnapshot, TaskCenterSummary,
};
pub use client_demo::{ClientDemoAdapter, ClientDemoState};
pub use client_flow::{
    active_center_actions, recommended_center, ClientCenterRoute, ClientFlowAdapter,
    ClientFlowState, ClientSelectionState,
};
pub use client_runtime_api::{ClientActionExecution, ClientRuntimeApi};
pub use client_shell::{
    ClientNotification, ClientNotificationLevel, ClientShellAdapter, ClientShellState,
};
pub use client_view::{build_client_shell_view_model, ClientShellBadge, ClientShellViewModel};
pub use loop_runner::{TaskAction, TaskRunReport, TaskScript};
pub use memory::{MemoryKind, MemoryRecord, MemoryStore};
pub use model::{Artifact, ChannelObject, Session, Task, TaskHost, TaskKind, TaskPriority, WorkItem};
pub use policy::{
    ApprovalRiskLevel, ExecPlan, FileWritePlan, PermissionDecision, WorkspaceExecutionPolicy,
    DEFAULT_HEARTBEAT_INTERVAL_MS,
};
pub use persistence::{PersistenceError, RuntimeStateStore};
pub use presentation::{
    build_approval_center_summary, build_approval_group_buckets,
    build_approval_history_entries, build_approval_surface_entries, build_channel_inbox_entries,
    build_channel_host_surface_entries, build_channel_source_counts,
    build_client_surface_snapshot, build_provider_surface_entries,
    build_session_surface_entries, build_task_board_entry, build_task_board_snapshot,
    build_wait_bucket_counts, build_work_item_surface_entries, apply_approval_history_window, build_approval_view_presets,
    approval_id_for, approval_risk_label, channel_host_kind_label, channel_host_runtime_key, connector_sync_mode_label, host_capability_labels, host_label_for_task_host,
    ApprovalCenterSummary, ApprovalGroupBucket, ApprovalHistoryEntry, ApprovalHistoryWindow,
    ApprovalSurfaceEntry, ApprovalViewPreset, DEFAULT_APPROVAL_HISTORY_LIMIT,
    ChannelHostSurfaceEntry, ChannelInboxEntry, ChannelSourceCount, ClientSurfaceSnapshot,
    ProviderSurfaceEntry, SessionSurfaceEntry, TaskAttentionLane, TaskBoardEntry,
    TaskBoardHostCount, TaskBoardSnapshot, TaskBoardSummary, WaitBucketCount,
    WorkItemSurfaceEntry,
};
pub use provider::{ProviderCatalog, ProviderConfig};
pub use registry::{
    RegisteredSkill, RegisteredSkillSource, SkillRegistry, ToolDefinition, ToolExecutionKind,
    ToolRegistry,
};
pub use runtime::{
    ApprovalBatchFailure, ApprovalBatchResult, ConnectorBatchFailure, ConnectorBatchResult,
    TaskBatchFailure, TaskBatchResult, TaskLoopRuntime,
};
pub use runtime_log::{RuntimeLogEntry, RuntimeLogLevel};
pub use scheduler::{QueuedTask, ScheduledWakeup, TaskQueue};
pub use skill_gating::{
    evaluate_load_time_gating, evaluate_request_time_gating, GatingContext, SkillGatingReport,
};
pub use state::{TaskHealthState, TaskProjection, TaskWaitKind, TaskWorkflowMode};
