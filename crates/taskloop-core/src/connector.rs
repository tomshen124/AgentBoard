use crate::{
    contracts::ConnectorSyncMode,
    model::{ChannelObject, Task, TaskHost, TaskKind, TaskPriority, WorkItem},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelHostKind {
    TaskList,
    DocumentList,
    BoardList,
    Inbox,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelHostMetadata {
    pub source_system: String,
    pub object_type: String,
    pub host_kind: ChannelHostKind,
    pub display_name: String,
    pub sync_mode: ConnectorSyncMode,
    pub supports_task_creation: bool,
    pub supports_status_sync: bool,
    pub supports_comment_sync: bool,
    pub supports_attachment_export: bool,
    pub requires_bidirectional_binding: bool,
    pub supports_writeback: bool,
    pub supports_background_sync: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelObjectPayload {
    pub source_system: String,
    pub object_type: String,
    pub external_id: String,
    pub workspace_id: Option<String>,
    pub title: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemInput {
    pub source_system: String,
    pub summary: String,
    pub channel_object_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorEnvelope {
    pub channel_object: ChannelObject,
    pub work_item: WorkItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectorSample;

impl ConnectorSample {
    pub fn normalize_external_task(
        &self,
        object_id: impl Into<String>,
        work_item_id: impl Into<String>,
        payload: ChannelObjectPayload,
    ) -> ConnectorEnvelope {
        let channel_object = ChannelObject {
            id: object_id.into(),
            source_system: payload.source_system.clone(),
            object_type: payload.object_type,
            external_id: payload.external_id,
            workspace_id: payload.workspace_id,
        };
        let work_item = WorkItem {
            id: work_item_id.into(),
            source_system: payload.source_system,
            summary: payload
                .body
                .map(|body| format!("{}\n\n{}", payload.title, body))
                .unwrap_or(payload.title),
            channel_object_id: Some(channel_object.id.clone()),
            status: "open".into(),
        };

        ConnectorEnvelope {
            channel_object,
            work_item,
        }
    }

    pub fn create_task_from_work_item(
        &self,
        task_id: impl Into<String>,
        session_id: impl Into<String>,
        work_item: &WorkItem,
        channel_object: &ChannelObject,
        created_at_ms: u64,
    ) -> Task {
        Task {
            id: task_id.into(),
            session_id: session_id.into(),
            work_item_id: Some(work_item.id.clone()),
            title: work_item.summary.lines().next().unwrap_or("External Task").to_string(),
            kind: TaskKind::ExternalHost,
            host: TaskHost::External {
                system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                object_id: channel_object.external_id.clone(),
            },
            priority: TaskPriority::Normal,
            created_at_ms,
        }
    }

    pub fn host_metadata_for(&self, channel_object: &ChannelObject) -> ChannelHostMetadata {
        match (
            channel_object.source_system.as_str(),
            channel_object.object_type.as_str(),
        ) {
            ("feishu", "task") => ChannelHostMetadata {
                source_system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                host_kind: ChannelHostKind::TaskList,
                display_name: "Feishu Tasks".into(),
                sync_mode: ConnectorSyncMode::Bidirectional,
                supports_task_creation: true,
                supports_status_sync: true,
                supports_comment_sync: true,
                supports_attachment_export: true,
                requires_bidirectional_binding: true,
                supports_writeback: true,
                supports_background_sync: true,
            },
            ("feishu", "doc") => ChannelHostMetadata {
                source_system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                host_kind: ChannelHostKind::DocumentList,
                display_name: "Feishu Docs".into(),
                sync_mode: ConnectorSyncMode::Bidirectional,
                supports_task_creation: false,
                supports_status_sync: false,
                supports_comment_sync: true,
                supports_attachment_export: true,
                requires_bidirectional_binding: false,
                supports_writeback: true,
                supports_background_sync: true,
            },
            ("feishu", "board") | ("feishu", "card") => ChannelHostMetadata {
                source_system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                host_kind: ChannelHostKind::BoardList,
                display_name: "Feishu Boards".into(),
                sync_mode: ConnectorSyncMode::Bidirectional,
                supports_task_creation: true,
                supports_status_sync: true,
                supports_comment_sync: true,
                supports_attachment_export: false,
                requires_bidirectional_binding: true,
                supports_writeback: true,
                supports_background_sync: true,
            },
            ("jira", "issue") => ChannelHostMetadata {
                source_system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                host_kind: ChannelHostKind::BoardList,
                display_name: "Jira Issues".into(),
                sync_mode: ConnectorSyncMode::Bidirectional,
                supports_task_creation: true,
                supports_status_sync: true,
                supports_comment_sync: true,
                supports_attachment_export: true,
                requires_bidirectional_binding: true,
                supports_writeback: true,
                supports_background_sync: true,
            },
            _ => ChannelHostMetadata {
                source_system: channel_object.source_system.clone(),
                object_type: channel_object.object_type.clone(),
                host_kind: ChannelHostKind::Inbox,
                display_name: format!(
                    "{} {}",
                    title_case(&channel_object.source_system),
                    title_case(&channel_object.object_type)
                ),
                sync_mode: ConnectorSyncMode::Inbound,
                supports_task_creation: false,
                supports_status_sync: false,
                supports_comment_sync: false,
                supports_attachment_export: false,
                requires_bidirectional_binding: false,
                supports_writeback: false,
                supports_background_sync: true,
            },
        }
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ChannelHostKind, ChannelObjectPayload, ConnectorSample};

    #[test]
    fn connector_sample_normalizes_external_task() {
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

        assert_eq!(envelope.channel_object.source_system, "feishu");
        assert_eq!(envelope.work_item.channel_object_id.as_deref(), Some("obj-1"));
        assert!(envelope.work_item.summary.contains("Prepare report"));
    }

    #[test]
    fn connector_sample_derives_feishu_task_host_metadata() {
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
                body: None,
            },
        );

        let metadata = connector.host_metadata_for(&envelope.channel_object);
        assert_eq!(metadata.display_name, "Feishu Tasks");
        assert_eq!(metadata.host_kind, ChannelHostKind::TaskList);
        assert_eq!(metadata.sync_mode, crate::contracts::ConnectorSyncMode::Bidirectional);
        assert!(metadata.supports_task_creation);
        assert!(metadata.supports_status_sync);
        assert!(metadata.supports_comment_sync);
        assert!(metadata.supports_attachment_export);
        assert!(metadata.requires_bidirectional_binding);
        assert!(metadata.supports_writeback);
        assert!(metadata.supports_background_sync);
    }
}
