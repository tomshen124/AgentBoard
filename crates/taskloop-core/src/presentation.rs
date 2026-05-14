use std::collections::BTreeMap;

use crate::{
    connector::ChannelHostKind,
    contracts::ConnectorSyncMode,
    contracts::{ApiFamily, HeaderPolicy, ProviderCapability, ProviderProtocol},
    model::TaskHost,
    policy::ApprovalRiskLevel,
    state::{TaskHealthState, TaskLifecycleState, TaskProjection, TaskWaitKind, TaskWorkflowMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAttentionLane {
    Foreground,
    Background,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardEntry {
    pub task_id: String,
    pub session_id: String,
    pub title: String,
    pub host_label: String,
    pub lifecycle: TaskLifecycleState,
    pub workflow_mode: TaskWorkflowMode,
    pub health: TaskHealthState,
    pub attention_lane: TaskAttentionLane,
    pub waiting_on: Option<TaskWaitKind>,
    pub blocked_reason: Option<String>,
    pub approval_action: Option<String>,
    pub approval_risk: Option<ApprovalRiskLevel>,
    pub approval_requested_at_ms: Option<u64>,
    pub resume_checkpoint: Option<String>,
    pub phase: Option<String>,
    pub current_step: Option<String>,
    pub total_steps: Option<u32>,
    pub progress_text: Option<String>,
    pub active_subagents: u32,
    pub is_queued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardHostCount {
    pub host_label: String,
    pub task_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardSummary {
    pub total_tasks: u32,
    pub foreground_tasks: u32,
    pub background_tasks: u32,
    pub queued_tasks: u32,
    pub running_tasks: u32,
    pub waiting_tasks: u32,
    pub complex_tasks: u32,
    pub stalled_tasks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardSnapshot {
    pub generated_at_ms: u64,
    pub summary: TaskBoardSummary,
    pub host_counts: Vec<TaskBoardHostCount>,
    pub entries: Vec<TaskBoardEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSurfaceEntry {
    pub session_id: String,
    pub title: String,
    pub total_tasks: u32,
    pub foreground_tasks: u32,
    pub background_tasks: u32,
    pub running_tasks: u32,
    pub waiting_tasks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelSourceCount {
    pub source_system: String,
    pub object_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSurfaceSnapshot {
    pub generated_at_ms: u64,
    pub total_sessions: u32,
    pub total_channel_objects: u32,
    pub total_work_items: u32,
    pub task_board: TaskBoardSnapshot,
    pub sessions: Vec<SessionSurfaceEntry>,
    pub channel_sources: Vec<ChannelSourceCount>,
    pub providers: Vec<ProviderSurfaceEntry>,
    pub waiting_buckets: Vec<WaitBucketCount>,
    pub work_items: Vec<WorkItemSurfaceEntry>,
    pub approval_summary: ApprovalCenterSummary,
    pub approval_view_presets: Vec<ApprovalViewPreset>,
    pub approval_queue: Vec<ApprovalSurfaceEntry>,
    pub approval_history_window: ApprovalHistoryWindow,
    pub approval_history: Vec<ApprovalHistoryEntry>,
    pub approval_groups: Vec<ApprovalGroupBucket>,
    pub channel_hosts: Vec<ChannelHostSurfaceEntry>,
    pub channel_inbox: Vec<ChannelInboxEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSurfaceEntry {
    pub profile: String,
    pub provider_id: String,
    pub protocol_label: String,
    pub api_family_label: String,
    pub header_policy_label: String,
    pub model_count: u32,
    pub capability_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitBucketCount {
    pub wait_kind_label: String,
    pub task_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemSurfaceEntry {
    pub work_item_id: String,
    pub source_system: String,
    pub status: String,
    pub summary: String,
    pub channel_object_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSurfaceEntry {
    pub approval_id: String,
    pub task_id: String,
    pub session_id: String,
    pub title: String,
    pub host_label: String,
    pub workflow_mode: TaskWorkflowMode,
    pub approval_action: Option<String>,
    pub approval_risk_label: Option<String>,
    pub approval_status: String,
    pub requested_at_ms: Option<u64>,
    pub sort_key: String,
    pub resume_checkpoint: Option<String>,
    pub reason: Option<String>,
    pub attention_lane: TaskAttentionLane,
    pub primary_action: String,
    pub secondary_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalHistoryEntry {
    pub approval_id: String,
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub approval_action: String,
    pub approval_risk_label: String,
    pub decision_source: String,
    pub resolved_by: String,
    pub resolved_at_ms: u64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCenterSummary {
    pub pending_total: u32,
    pub pending_high_risk: u32,
    pub approved_total: u32,
    pub rejected_total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalGroupBucket {
    pub bucket_key: String,
    pub status: String,
    pub approval_action: String,
    pub approval_risk_label: String,
    pub item_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalViewPreset {
    pub view_key: String,
    pub label: String,
    pub item_count: u32,
    pub status_filter: String,
    pub risk_filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalHistoryWindow {
    pub total_items: u32,
    pub visible_items: u32,
    pub limit: u32,
}

pub const DEFAULT_APPROVAL_HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelInboxEntry {
    pub work_item_id: String,
    pub source_system: String,
    pub object_type: String,
    pub host_kind: String,
    pub host_display_name: String,
    pub sync_mode_label: String,
    pub capability_labels: Vec<String>,
    pub requires_bidirectional_binding: bool,
    pub external_id: String,
    pub workspace_id: Option<String>,
    pub status: String,
    pub summary: String,
    pub linked_task_id: Option<String>,
    pub linked_task_lifecycle: Option<String>,
    pub linked_task_waiting_on: Option<String>,
    pub needs_attention: bool,
    pub supports_writeback: bool,
    pub supports_background_sync: bool,
    pub primary_action: String,
    pub secondary_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelHostSurfaceEntry {
    pub host_key: String,
    pub source_system: String,
    pub workspace_id: Option<String>,
    pub object_type: String,
    pub host_kind: String,
    pub host_display_name: String,
    pub sync_mode_label: String,
    pub capability_labels: Vec<String>,
    pub requires_bidirectional_binding: bool,
    pub display_label: String,
    pub item_count: u32,
    pub attention_count: u32,
    pub open_count: u32,
    pub supports_writeback: bool,
    pub supports_background_sync: bool,
    pub primary_action: String,
    pub secondary_actions: Vec<String>,
}

pub fn build_task_board_entry(
    projection: &TaskProjection,
    host_label: String,
    attention_lane: TaskAttentionLane,
    is_queued: bool,
) -> TaskBoardEntry {
    TaskBoardEntry {
        task_id: projection.task_id.clone(),
        session_id: projection.session_id.clone(),
        title: projection.title.clone(),
        host_label,
        lifecycle: projection.lifecycle.clone(),
        workflow_mode: projection.workflow_mode.clone(),
        health: projection.health.clone(),
        attention_lane,
        waiting_on: projection.waiting_on.clone(),
        blocked_reason: projection.blocked_reason.clone(),
        approval_action: projection.approval_action.clone(),
        approval_risk: projection.approval_risk.clone(),
        approval_requested_at_ms: projection.approval_requested_at_ms,
        resume_checkpoint: projection.resume_checkpoint.clone(),
        phase: projection.phase.clone(),
        current_step: projection.current_step.clone(),
        total_steps: projection.total_steps,
        progress_text: projection.progress_text.clone(),
        active_subagents: projection.active_subagents,
        is_queued,
    }
}

pub fn host_label_for_task_host(host: &TaskHost) -> String {
    match host {
        TaskHost::Internal => "agentboard".into(),
        TaskHost::External { system, .. } => system.clone(),
    }
}

pub fn build_task_board_snapshot(
    generated_at_ms: u64,
    entries: Vec<TaskBoardEntry>,
) -> TaskBoardSnapshot {
    let mut host_counts = BTreeMap::<String, u32>::new();
    let mut summary = TaskBoardSummary {
        total_tasks: entries.len() as u32,
        foreground_tasks: 0,
        background_tasks: 0,
        queued_tasks: 0,
        running_tasks: 0,
        waiting_tasks: 0,
        complex_tasks: 0,
        stalled_tasks: 0,
    };

    for entry in &entries {
        *host_counts.entry(entry.host_label.clone()).or_insert(0) += 1;
        match entry.attention_lane {
            TaskAttentionLane::Foreground => summary.foreground_tasks += 1,
            TaskAttentionLane::Background => summary.background_tasks += 1,
        }
        if entry.is_queued {
            summary.queued_tasks += 1;
        }
        match entry.lifecycle {
            TaskLifecycleState::Running => summary.running_tasks += 1,
            TaskLifecycleState::Waiting => summary.waiting_tasks += 1,
            _ => {}
        }
        if matches!(entry.workflow_mode, TaskWorkflowMode::Complex) {
            summary.complex_tasks += 1;
        }
        if matches!(entry.health, TaskHealthState::Stalled) {
            summary.stalled_tasks += 1;
        }
    }

    let host_counts = host_counts
        .into_iter()
        .map(|(host_label, task_count)| TaskBoardHostCount {
            host_label,
            task_count,
        })
        .collect();

    TaskBoardSnapshot {
        generated_at_ms,
        summary,
        host_counts,
        entries,
    }
}

pub fn build_session_surface_entries(
    sessions: Vec<(String, String)>,
    task_entries: &[TaskBoardEntry],
) -> Vec<SessionSurfaceEntry> {
    let mut entries = sessions
        .into_iter()
        .map(|(session_id, title)| {
            let related = task_entries
                .iter()
                .filter(|entry| entry.session_id == session_id)
                .collect::<Vec<_>>();
            SessionSurfaceEntry {
                session_id,
                title,
                total_tasks: related.len() as u32,
                foreground_tasks: related
                    .iter()
                    .filter(|entry| matches!(entry.attention_lane, TaskAttentionLane::Foreground))
                    .count() as u32,
                background_tasks: related
                    .iter()
                    .filter(|entry| matches!(entry.attention_lane, TaskAttentionLane::Background))
                    .count() as u32,
                running_tasks: related
                    .iter()
                    .filter(|entry| matches!(entry.lifecycle, TaskLifecycleState::Running))
                    .count() as u32,
                waiting_tasks: related
                    .iter()
                    .filter(|entry| matches!(entry.lifecycle, TaskLifecycleState::Waiting))
                    .count() as u32,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    entries
}

pub fn build_channel_source_counts(
    channel_objects: Vec<String>,
) -> Vec<ChannelSourceCount> {
    let mut counts = BTreeMap::<String, u32>::new();
    for source_system in channel_objects {
        *counts.entry(source_system).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(source_system, object_count)| ChannelSourceCount {
            source_system,
            object_count,
        })
        .collect()
}

pub fn build_client_surface_snapshot(
    generated_at_ms: u64,
    task_board: TaskBoardSnapshot,
    sessions: Vec<SessionSurfaceEntry>,
    channel_sources: Vec<ChannelSourceCount>,
    providers: Vec<ProviderSurfaceEntry>,
    waiting_buckets: Vec<WaitBucketCount>,
    work_items: Vec<WorkItemSurfaceEntry>,
    approval_summary: ApprovalCenterSummary,
    approval_view_presets: Vec<ApprovalViewPreset>,
    approval_queue: Vec<ApprovalSurfaceEntry>,
    approval_history_window: ApprovalHistoryWindow,
    approval_history: Vec<ApprovalHistoryEntry>,
    approval_groups: Vec<ApprovalGroupBucket>,
    channel_hosts: Vec<ChannelHostSurfaceEntry>,
    channel_inbox: Vec<ChannelInboxEntry>,
    total_work_items: u32,
) -> ClientSurfaceSnapshot {
    ClientSurfaceSnapshot {
        generated_at_ms,
        total_sessions: sessions.len() as u32,
        total_channel_objects: channel_sources.iter().map(|bucket| bucket.object_count).sum(),
        total_work_items,
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
    }
}

pub fn build_provider_surface_entries(
    providers: Vec<(
        String,
        String,
        ProviderProtocol,
        ApiFamily,
        HeaderPolicy,
        usize,
        Vec<ProviderCapability>,
    )>,
) -> Vec<ProviderSurfaceEntry> {
    let mut entries = providers
        .into_iter()
        .map(
            |(
                profile,
                provider_id,
                protocol,
                api_family,
                header_policy,
                model_count,
                capabilities,
            )| ProviderSurfaceEntry {
                profile,
                provider_id,
                protocol_label: protocol_label(&protocol),
                api_family_label: api_family_label(&api_family),
                header_policy_label: header_policy_label(&header_policy),
                model_count: model_count as u32,
                capability_labels: capabilities
                    .iter()
                    .map(provider_capability_label)
                    .collect(),
            },
        )
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.profile.cmp(&right.profile));
    entries
}

fn protocol_label(value: &ProviderProtocol) -> String {
    match value {
        ProviderProtocol::OpenAiCompatible => "openai-compatible".into(),
        ProviderProtocol::AnthropicCompatible => "anthropic-compatible".into(),
        ProviderProtocol::Local => "local".into(),
        ProviderProtocol::Custom(value) => value.clone(),
    }
}

fn api_family_label(value: &ApiFamily) -> String {
    match value {
        ApiFamily::Responses => "responses".into(),
        ApiFamily::ChatCompletions => "chat-completions".into(),
        ApiFamily::AnthropicMessages => "anthropic-messages".into(),
        ApiFamily::Custom(value) => value.clone(),
    }
}

fn header_policy_label(value: &HeaderPolicy) -> String {
    match value {
        HeaderPolicy::Strict => "strict".into(),
        HeaderPolicy::Compatible => "compatible".into(),
        HeaderPolicy::Extended => "extended".into(),
    }
}

fn provider_capability_label(value: &ProviderCapability) -> String {
    match value {
        ProviderCapability::Streaming => "streaming".into(),
        ProviderCapability::ToolCalling => "tool-calling".into(),
        ProviderCapability::JsonOutput => "json-output".into(),
        ProviderCapability::ImageInput => "image-input".into(),
        ProviderCapability::Reasoning => "reasoning".into(),
        ProviderCapability::WebSearch => "web-search".into(),
        ProviderCapability::Custom(value) => value.clone(),
    }
}

pub fn build_wait_bucket_counts(entries: &[TaskBoardEntry]) -> Vec<WaitBucketCount> {
    let mut counts = BTreeMap::<String, u32>::new();
    for entry in entries {
        if let Some(wait_kind) = &entry.waiting_on {
            *counts.entry(wait_kind_label(wait_kind)).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(wait_kind_label, task_count)| WaitBucketCount {
            wait_kind_label,
            task_count,
        })
        .collect()
}

pub fn build_work_item_surface_entries(
    work_items: Vec<(String, String, String, String, Option<String>)>,
) -> Vec<WorkItemSurfaceEntry> {
    let mut entries = work_items
        .into_iter()
        .map(
            |(work_item_id, source_system, status, summary, channel_object_id)| WorkItemSurfaceEntry {
                work_item_id,
                source_system,
                status,
                summary,
                channel_object_id,
            },
        )
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.work_item_id.cmp(&right.work_item_id));
    entries
}

pub fn build_approval_surface_entries(entries: &[TaskBoardEntry]) -> Vec<ApprovalSurfaceEntry> {
    let mut approvals = entries
        .iter()
        .filter(|entry| matches!(entry.waiting_on, Some(TaskWaitKind::Approval)))
        .map(|entry| ApprovalSurfaceEntry {
            approval_id: approval_id_for(
                &entry.task_id,
                entry.approval_action.as_deref(),
                entry.resume_checkpoint.as_deref(),
            ),
            task_id: entry.task_id.clone(),
            session_id: entry.session_id.clone(),
            title: entry.title.clone(),
            host_label: entry.host_label.clone(),
            workflow_mode: entry.workflow_mode.clone(),
            approval_action: entry.approval_action.clone(),
            approval_risk_label: entry
                .approval_risk
                .as_ref()
                .map(|value| approval_risk_label(value)),
            approval_status: "pending".into(),
            requested_at_ms: entry.approval_requested_at_ms,
            sort_key: approval_sort_key(
                entry.approval_risk.as_ref(),
                entry.approval_requested_at_ms,
                &entry.task_id,
            ),
            resume_checkpoint: entry.resume_checkpoint.clone(),
            reason: entry
                .blocked_reason
                .clone()
                .or_else(|| entry.progress_text.clone()),
            attention_lane: entry.attention_lane.clone(),
            primary_action: "open_approval".into(),
            secondary_actions: vec!["open_task".into(), "inspect_policy".into()],
        })
        .collect::<Vec<_>>();
    approvals.sort_by(|left, right| left.sort_key.cmp(&right.sort_key));
    approvals
}

pub fn build_approval_history_entries(
    items: Vec<(String, String, String, String, String, String, String, String, u64, Option<String>)>,
) -> Vec<ApprovalHistoryEntry> {
    let mut entries = items
        .into_iter()
        .map(
            |(
                approval_id,
                task_id,
                title,
                status,
                approval_action,
                approval_risk_label,
                decision_source,
                resolved_by,
                resolved_at_ms,
                reason,
            )| ApprovalHistoryEntry {
                approval_id,
                task_id,
                title,
                status,
                approval_action,
                approval_risk_label,
                decision_source,
                resolved_by,
                resolved_at_ms,
                reason,
            },
        )
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .resolved_at_ms
            .cmp(&left.resolved_at_ms)
            .then_with(|| left.approval_id.cmp(&right.approval_id))
    });
    entries
}

pub fn build_approval_center_summary(
    approval_queue: &[ApprovalSurfaceEntry],
    approval_history: &[ApprovalHistoryEntry],
) -> ApprovalCenterSummary {
    ApprovalCenterSummary {
        pending_total: approval_queue.len() as u32,
        pending_high_risk: approval_queue
            .iter()
            .filter(|item| item.approval_risk_label.as_deref() == Some("high"))
            .count() as u32,
        approved_total: approval_history
            .iter()
            .filter(|item| item.status == "approved")
            .count() as u32,
        rejected_total: approval_history
            .iter()
            .filter(|item| item.status == "rejected")
            .count() as u32,
    }
}

pub fn build_approval_group_buckets(
    approval_queue: &[ApprovalSurfaceEntry],
    approval_history: &[ApprovalHistoryEntry],
) -> Vec<ApprovalGroupBucket> {
    let mut buckets = BTreeMap::<(String, String, String), u32>::new();

    for item in approval_queue {
        let key = (
            item.approval_status.clone(),
            item.approval_action.clone().unwrap_or_else(|| "unknown".into()),
            item.approval_risk_label
                .clone()
                .unwrap_or_else(|| "unknown".into()),
        );
        *buckets.entry(key).or_insert(0) += 1;
    }

    for item in approval_history {
        let key = (
            item.status.clone(),
            item.approval_action.clone(),
            item.approval_risk_label.clone(),
        );
        *buckets.entry(key).or_insert(0) += 1;
    }

    buckets
        .into_iter()
        .map(|((status, approval_action, approval_risk_label), item_count)| ApprovalGroupBucket {
            bucket_key: format!("{status}:{approval_action}:{approval_risk_label}"),
            status,
            approval_action,
            approval_risk_label,
            item_count,
        })
        .collect()
}

pub fn build_approval_view_presets(
    approval_summary: &ApprovalCenterSummary,
    approval_history: &[ApprovalHistoryEntry],
) -> Vec<ApprovalViewPreset> {
    vec![
        ApprovalViewPreset {
            view_key: "pending_high_risk".into(),
            label: "Pending High Risk".into(),
            item_count: approval_summary.pending_high_risk,
            status_filter: "pending".into(),
            risk_filter: Some("high".into()),
        },
        ApprovalViewPreset {
            view_key: "pending_all".into(),
            label: "Pending All".into(),
            item_count: approval_summary.pending_total,
            status_filter: "pending".into(),
            risk_filter: None,
        },
        ApprovalViewPreset {
            view_key: "approved_recent".into(),
            label: "Approved".into(),
            item_count: approval_history
                .iter()
                .filter(|item| item.status == "approved")
                .count() as u32,
            status_filter: "approved".into(),
            risk_filter: None,
        },
        ApprovalViewPreset {
            view_key: "rejected_recent".into(),
            label: "Rejected".into(),
            item_count: approval_history
                .iter()
                .filter(|item| item.status == "rejected")
                .count() as u32,
            status_filter: "rejected".into(),
            risk_filter: None,
        },
    ]
}

pub fn apply_approval_history_window(
    mut approval_history: Vec<ApprovalHistoryEntry>,
    limit: usize,
) -> (ApprovalHistoryWindow, Vec<ApprovalHistoryEntry>) {
    let total_items = approval_history.len() as u32;
    approval_history.sort_by(|left, right| {
        right
            .resolved_at_ms
            .cmp(&left.resolved_at_ms)
            .then_with(|| left.approval_id.cmp(&right.approval_id))
    });
    approval_history.truncate(limit);
    let visible_items = approval_history.len() as u32;
    (
        ApprovalHistoryWindow {
            total_items,
            visible_items,
            limit: limit as u32,
        },
        approval_history,
    )
}

pub fn build_channel_inbox_entries(
    items: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Vec<String>,
        bool,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
        Option<TaskLifecycleState>,
        Option<TaskWaitKind>,
        bool,
        bool,
        bool,
    )>,
) -> Vec<ChannelInboxEntry> {
    let mut entries = items
        .into_iter()
        .map(|(
            work_item_id,
            source_system,
            object_type,
            host_kind,
            host_display_name,
            sync_mode_label,
            capability_labels,
            requires_bidirectional_binding,
            external_id,
            workspace_id,
            status,
            summary,
            linked_task_id,
            linked_task_lifecycle,
            linked_task_waiting_on,
            needs_attention,
            supports_writeback,
            supports_background_sync,
        )| {
            let has_linked_task = linked_task_id.is_some();
            ChannelInboxEntry {
                work_item_id,
                source_system,
                object_type,
                host_kind,
                host_display_name,
                sync_mode_label,
                capability_labels,
                requires_bidirectional_binding,
                external_id,
                workspace_id,
                status,
                summary,
                linked_task_id,
                linked_task_lifecycle: linked_task_lifecycle
                    .map(|value| lifecycle_label(&value)),
                linked_task_waiting_on: linked_task_waiting_on
                    .map(|value| wait_kind_label(&value)),
                needs_attention,
                supports_writeback,
                supports_background_sync,
                primary_action: if needs_attention {
                    "open_linked_task".into()
                } else {
                    "open_work_item".into()
                },
                secondary_actions: channel_inbox_secondary_actions(has_linked_task),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.source_system
            .cmp(&right.source_system)
            .then_with(|| left.work_item_id.cmp(&right.work_item_id))
    });
    entries
}

pub fn build_channel_host_surface_entries(
    items: &[ChannelInboxEntry],
) -> Vec<ChannelHostSurfaceEntry> {
    let mut buckets = BTreeMap::<
        (
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            Vec<String>,
            bool,
            bool,
            bool,
        ),
        (u32, u32, u32),
    >::new();
    for item in items {
        let key = (
            item.source_system.clone(),
            item.workspace_id.clone(),
            item.object_type.clone(),
            item.host_kind.clone(),
            item.host_display_name.clone(),
            item.sync_mode_label.clone(),
            item.capability_labels.clone(),
            item.requires_bidirectional_binding,
            item.supports_writeback,
            item.supports_background_sync,
        );
        let entry = buckets.entry(key).or_insert((0, 0, 0));
        entry.0 += 1;
        if item.needs_attention {
            entry.1 += 1;
        }
        if item.status == "open" {
            entry.2 += 1;
        }
    }

    let mut entries = buckets
        .into_iter()
        .map(
            |(
                (
                    source_system,
                    workspace_id,
                    object_type,
                    host_kind,
                    host_display_name,
                    sync_mode_label,
                    capability_labels,
                    requires_bidirectional_binding,
                    supports_writeback,
                    supports_background_sync,
                ),
                (item_count, attention_count, open_count),
            )| {
                ChannelHostSurfaceEntry {
                    host_key: channel_host_runtime_key(
                        &source_system,
                        workspace_id.as_deref(),
                        &object_type,
                    ),
                    host_kind,
                    host_display_name,
                    sync_mode_label,
                    capability_labels,
                    requires_bidirectional_binding,
                    display_label: channel_host_display_label(
                        &source_system,
                        workspace_id.as_deref(),
                        &object_type,
                    ),
                    source_system,
                    workspace_id,
                    object_type,
                    item_count,
                    attention_count,
                    open_count,
                    supports_writeback,
                    supports_background_sync,
                    primary_action: "open_host_list".into(),
                    secondary_actions: vec!["sync_host".into(), "open_host_settings".into()],
                }
            },
        )
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        right
            .attention_count
            .cmp(&left.attention_count)
            .then_with(|| right.open_count.cmp(&left.open_count))
            .then_with(|| left.source_system.cmp(&right.source_system))
            .then_with(|| left.object_type.cmp(&right.object_type))
            .then_with(|| left.workspace_id.cmp(&right.workspace_id))
    });
    entries
}

fn wait_kind_label(value: &TaskWaitKind) -> String {
    match value {
        TaskWaitKind::User => "user".into(),
        TaskWaitKind::Approval => "approval".into(),
        TaskWaitKind::Tool => "tool".into(),
        TaskWaitKind::Schedule => "schedule".into(),
        TaskWaitKind::Subtask => "subtask".into(),
    }
}

fn lifecycle_label(value: &TaskLifecycleState) -> String {
    match value {
        TaskLifecycleState::Queued => "queued".into(),
        TaskLifecycleState::Running => "running".into(),
        TaskLifecycleState::Waiting => "waiting".into(),
        TaskLifecycleState::Paused => "paused".into(),
        TaskLifecycleState::Done => "done".into(),
        TaskLifecycleState::Failed => "failed".into(),
        TaskLifecycleState::Cancelled => "cancelled".into(),
    }
}

pub fn approval_risk_label(value: &crate::policy::ApprovalRiskLevel) -> String {
    match value {
        crate::policy::ApprovalRiskLevel::Low => "low".into(),
        crate::policy::ApprovalRiskLevel::High => "high".into(),
    }
}

fn approval_sort_key(
    risk: Option<&crate::policy::ApprovalRiskLevel>,
    requested_at_ms: Option<u64>,
    task_id: &str,
) -> String {
    let risk_rank = match risk {
        Some(crate::policy::ApprovalRiskLevel::High) => 0_u8,
        _ => 1_u8,
    };
    let requested_at_ms = requested_at_ms.unwrap_or(u64::MAX);
    format!("{risk_rank:01}:{requested_at_ms:020}:{task_id}")
}

fn channel_inbox_secondary_actions(has_linked_task: bool) -> Vec<String> {
    let mut actions = vec!["open_channel_object".into(), "sync_external".into()];
    if has_linked_task {
        actions.insert(0, "open_linked_task".into());
    }
    actions
}

fn channel_host_display_label(
    source_system: &str,
    workspace_id: Option<&str>,
    object_type: &str,
) -> String {
    channel_host_runtime_key(source_system, workspace_id, object_type)
}

pub fn channel_host_runtime_key(
    source_system: &str,
    workspace_id: Option<&str>,
    object_type: &str,
) -> String {
    match workspace_id {
        Some(workspace_id) if !workspace_id.is_empty() => {
            format!("{source_system}:{workspace_id}:{object_type}")
        }
        _ => format!("{source_system}:{object_type}")
    }
}

pub fn channel_host_kind_label(value: &ChannelHostKind) -> String {
    match value {
        ChannelHostKind::TaskList => "task_list".into(),
        ChannelHostKind::DocumentList => "document_list".into(),
        ChannelHostKind::BoardList => "board_list".into(),
        ChannelHostKind::Inbox => "inbox".into(),
    }
}

pub fn approval_id_for(
    task_id: &str,
    action: Option<&str>,
    checkpoint: Option<&str>,
) -> String {
    format!(
        "approval:{}:{}:{}",
        task_id,
        action.unwrap_or("unknown"),
        checkpoint.unwrap_or("pending")
    )
}

pub fn connector_sync_mode_label(value: &ConnectorSyncMode) -> String {
    match value {
        ConnectorSyncMode::Inbound => "inbound".into(),
        ConnectorSyncMode::Outbound => "outbound".into(),
        ConnectorSyncMode::Bidirectional => "bidirectional".into(),
    }
}

pub fn host_capability_labels(
    supports_task_creation: bool,
    supports_status_sync: bool,
    supports_comment_sync: bool,
    supports_attachment_export: bool,
) -> Vec<String> {
    let mut labels = Vec::new();
    if supports_task_creation {
        labels.push("task_creation".into());
    }
    if supports_status_sync {
        labels.push("status_sync".into());
    }
    if supports_comment_sync {
        labels.push("comment_sync".into());
    }
    if supports_attachment_export {
        labels.push("attachment_export".into());
    }
    labels
}
