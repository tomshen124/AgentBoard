use crate::presentation::{
    ApprovalCenterSummary, ApprovalGroupBucket, ApprovalHistoryEntry, ApprovalHistoryWindow,
    ApprovalSurfaceEntry, ApprovalViewPreset, ChannelHostSurfaceEntry, ChannelInboxEntry,
    ChannelSourceCount, ClientSurfaceSnapshot, ProviderSurfaceEntry, SessionSurfaceEntry,
    TaskBoardSnapshot, WaitBucketCount, WorkItemSurfaceEntry,
};
use crate::runtime::{
    ApprovalBatchResult, ConnectorBatchResult, TaskBatchResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCenterSnapshot {
    pub summary: TaskCenterSummary,
    pub view_presets: Vec<CenterViewPreset>,
    pub available_actions: Vec<ClientActionDescriptor>,
    pub task_board: TaskBoardSnapshot,
    pub sessions: Vec<SessionSurfaceEntry>,
    pub waiting_buckets: Vec<WaitBucketCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCenterSnapshot {
    pub summary: ApprovalCenterSummary,
    pub view_presets: Vec<ApprovalViewPreset>,
    pub available_actions: Vec<ClientActionDescriptor>,
    pub queue: Vec<ApprovalSurfaceEntry>,
    pub history_window: ApprovalHistoryWindow,
    pub history: Vec<ApprovalHistoryEntry>,
    pub groups: Vec<ApprovalGroupBucket>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorCenterSnapshot {
    pub summary: ConnectorCenterSummary,
    pub view_presets: Vec<CenterViewPreset>,
    pub available_actions: Vec<ClientActionDescriptor>,
    pub channel_sources: Vec<ChannelSourceCount>,
    pub channel_hosts: Vec<ChannelHostSurfaceEntry>,
    pub channel_inbox: Vec<ChannelInboxEntry>,
    pub work_items: Vec<WorkItemSurfaceEntry>,
    pub providers: Vec<ProviderSurfaceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientApiSnapshot {
    pub generated_at_ms: u64,
    pub total_sessions: u32,
    pub total_channel_objects: u32,
    pub total_work_items: u32,
    pub task_center: TaskCenterSnapshot,
    pub approval_center: ApprovalCenterSnapshot,
    pub connector_center: ConnectorCenterSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientActionDescriptor {
    pub action_key: String,
    pub label: String,
    pub enabled: bool,
    pub bulk: bool,
    pub selection_required: bool,
    pub reason_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCenterKind {
    Task,
    Approval,
    Connector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientActionRequest {
    pub center: ClientCenterKind,
    pub action_key: String,
    pub item_keys: Vec<String>,
    pub reason: Option<String>,
    pub timestamp_ms: u64,
    pub background: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientActionResult {
    pub action_key: String,
    pub requested: u32,
    pub succeeded: Vec<String>,
    pub failed: Vec<ClientActionFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientActionFailure {
    pub item_key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterViewPreset {
    pub view_key: String,
    pub label: String,
    pub item_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCenterSummary {
    pub total_tasks: u32,
    pub foreground_tasks: u32,
    pub waiting_tasks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorCenterSummary {
    pub total_hosts: u32,
    pub attention_hosts: u32,
    pub syncable_hosts: u32,
    pub bidirectional_hosts: u32,
    pub total_inbox_items: u32,
}

pub fn build_client_api_snapshot(surface: ClientSurfaceSnapshot) -> ClientApiSnapshot {
    let task_summary = build_task_center_summary(&surface.task_board);
    let task_view_presets = build_task_center_view_presets(&task_summary);
    let task_actions = build_task_center_actions(&surface.task_board, &surface.waiting_buckets);
    let approval_actions = build_approval_center_actions(
        &surface.approval_summary,
        &surface.approval_history_window,
    );
    let connector_summary =
        build_connector_center_summary(&surface.channel_hosts, &surface.channel_inbox);
    let connector_view_presets = build_connector_center_view_presets(&connector_summary);
    let connector_actions = build_connector_center_actions(
        &surface.channel_hosts,
        &surface.channel_inbox,
    );
    ClientApiSnapshot {
        generated_at_ms: surface.generated_at_ms,
        total_sessions: surface.total_sessions,
        total_channel_objects: surface.total_channel_objects,
        total_work_items: surface.total_work_items,
        task_center: TaskCenterSnapshot {
            summary: task_summary,
            view_presets: task_view_presets,
            available_actions: task_actions,
            task_board: surface.task_board,
            sessions: surface.sessions,
            waiting_buckets: surface.waiting_buckets,
        },
        approval_center: ApprovalCenterSnapshot {
            summary: surface.approval_summary,
            view_presets: surface.approval_view_presets,
            available_actions: approval_actions,
            queue: surface.approval_queue,
            history_window: surface.approval_history_window,
            history: surface.approval_history,
            groups: surface.approval_groups,
        },
        connector_center: ConnectorCenterSnapshot {
            summary: connector_summary,
            view_presets: connector_view_presets,
            available_actions: connector_actions,
            channel_sources: surface.channel_sources,
            channel_hosts: surface.channel_hosts,
            channel_inbox: surface.channel_inbox,
            work_items: surface.work_items,
            providers: surface.providers,
        },
    }
}

pub fn build_approval_center_actions(
    summary: &ApprovalCenterSummary,
    history_window: &ApprovalHistoryWindow,
) -> Vec<ClientActionDescriptor> {
    vec![
        ClientActionDescriptor {
            action_key: "approve_selected".into(),
            label: "Approve Selected".into(),
            enabled: summary.pending_total > 0,
            bulk: true,
            selection_required: true,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "reject_selected".into(),
            label: "Reject Selected".into(),
            enabled: summary.pending_total > 0,
            bulk: true,
            selection_required: true,
            reason_required: true,
        },
        ClientActionDescriptor {
            action_key: "view_history".into(),
            label: "View History".into(),
            enabled: history_window.visible_items > 0,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
    ]
}

pub fn build_task_center_summary(task_board: &TaskBoardSnapshot) -> TaskCenterSummary {
    TaskCenterSummary {
        total_tasks: task_board.summary.total_tasks,
        foreground_tasks: task_board.summary.foreground_tasks,
        waiting_tasks: task_board.summary.waiting_tasks,
    }
}

pub fn build_task_center_view_presets(summary: &TaskCenterSummary) -> Vec<CenterViewPreset> {
    vec![
        CenterViewPreset {
            view_key: "foreground".into(),
            label: "Foreground".into(),
            item_count: summary.foreground_tasks,
        },
        CenterViewPreset {
            view_key: "waiting".into(),
            label: "Waiting".into(),
            item_count: summary.waiting_tasks,
        },
        CenterViewPreset {
            view_key: "all_tasks".into(),
            label: "All Tasks".into(),
            item_count: summary.total_tasks,
        },
    ]
}

pub fn build_task_center_actions(
    task_board: &TaskBoardSnapshot,
    waiting_buckets: &[WaitBucketCount],
) -> Vec<ClientActionDescriptor> {
    let has_tasks = task_board.summary.total_tasks > 0;
    let has_waiting = waiting_buckets.iter().any(|bucket| bucket.task_count > 0);
    vec![
        ClientActionDescriptor {
            action_key: "open_foreground_tasks".into(),
            label: "Open Foreground Tasks".into(),
            enabled: task_board.summary.foreground_tasks > 0,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "open_waiting_tasks".into(),
            label: "Open Waiting Tasks".into(),
            enabled: has_waiting,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "refresh_task_board".into(),
            label: "Refresh Task Board".into(),
            enabled: has_tasks,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "cancel_selected".into(),
            label: "Cancel Selected".into(),
            enabled: has_tasks,
            bulk: true,
            selection_required: true,
            reason_required: true,
        },
        ClientActionDescriptor {
            action_key: "requeue_selected".into(),
            label: "Requeue Selected".into(),
            enabled: has_tasks,
            bulk: true,
            selection_required: true,
            reason_required: false,
        },
    ]
}

pub fn build_connector_center_actions(
    channel_hosts: &[ChannelHostSurfaceEntry],
    channel_inbox: &[ChannelInboxEntry],
) -> Vec<ClientActionDescriptor> {
    let has_hosts = !channel_hosts.is_empty();
    let has_syncable_hosts = channel_hosts
        .iter()
        .any(|host| host.supports_background_sync || host.supports_writeback);
    let has_attention = channel_inbox.iter().any(|item| item.needs_attention);
    vec![
        ClientActionDescriptor {
            action_key: "open_channel_hosts".into(),
            label: "Open Channel Hosts".into(),
            enabled: has_hosts,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "open_attention_inbox".into(),
            label: "Open Attention Inbox".into(),
            enabled: has_attention,
            bulk: false,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "sync_all_hosts".into(),
            label: "Sync All Hosts".into(),
            enabled: has_syncable_hosts,
            bulk: true,
            selection_required: false,
            reason_required: false,
        },
        ClientActionDescriptor {
            action_key: "sync_selected_hosts".into(),
            label: "Sync Selected Hosts".into(),
            enabled: has_syncable_hosts,
            bulk: true,
            selection_required: true,
            reason_required: false,
        },
    ]
}

pub fn build_connector_center_summary(
    channel_hosts: &[ChannelHostSurfaceEntry],
    channel_inbox: &[ChannelInboxEntry],
) -> ConnectorCenterSummary {
    ConnectorCenterSummary {
        total_hosts: channel_hosts.len() as u32,
        attention_hosts: channel_hosts
            .iter()
            .filter(|host| host.attention_count > 0)
            .count() as u32,
        syncable_hosts: channel_hosts
            .iter()
            .filter(|host| host.supports_background_sync || host.supports_writeback)
            .count() as u32,
        bidirectional_hosts: channel_hosts
            .iter()
            .filter(|host| host.sync_mode_label == "bidirectional")
            .count() as u32,
        total_inbox_items: channel_inbox.len() as u32,
    }
}

pub fn build_connector_center_view_presets(
    summary: &ConnectorCenterSummary,
) -> Vec<CenterViewPreset> {
    vec![
        CenterViewPreset {
            view_key: "attention_hosts".into(),
            label: "Attention Hosts".into(),
            item_count: summary.attention_hosts,
        },
        CenterViewPreset {
            view_key: "syncable_hosts".into(),
            label: "Syncable Hosts".into(),
            item_count: summary.syncable_hosts,
        },
        CenterViewPreset {
            view_key: "bidirectional_hosts".into(),
            label: "Bidirectional Hosts".into(),
            item_count: summary.bidirectional_hosts,
        },
        CenterViewPreset {
            view_key: "all_hosts".into(),
            label: "All Hosts".into(),
            item_count: summary.total_hosts,
        },
        CenterViewPreset {
            view_key: "all_inbox".into(),
            label: "All Inbox".into(),
            item_count: summary.total_inbox_items,
        },
    ]
}

pub fn map_approval_batch_result(
    action_key: impl Into<String>,
    result: ApprovalBatchResult,
) -> ClientActionResult {
    ClientActionResult {
        action_key: action_key.into(),
        requested: result.requested,
        succeeded: result.succeeded,
        failed: result
            .failed
            .into_iter()
            .map(|item| ClientActionFailure {
                item_key: item.approval_id,
                reason: item.reason,
            })
            .collect(),
    }
}

pub fn map_task_batch_result(
    action_key: impl Into<String>,
    result: TaskBatchResult,
) -> ClientActionResult {
    ClientActionResult {
        action_key: action_key.into(),
        requested: result.requested,
        succeeded: result.succeeded,
        failed: result
            .failed
            .into_iter()
            .map(|item| ClientActionFailure {
                item_key: item.task_id,
                reason: item.reason,
            })
            .collect(),
    }
}

pub fn map_connector_batch_result(
    action_key: impl Into<String>,
    result: ConnectorBatchResult,
) -> ClientActionResult {
    ClientActionResult {
        action_key: action_key.into(),
        requested: result.requested,
        succeeded: result.succeeded,
        failed: result
            .failed
            .into_iter()
            .map(|item| ClientActionFailure {
                item_key: item.host_key,
                reason: item.reason,
            })
            .collect(),
    }
}
