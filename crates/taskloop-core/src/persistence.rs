use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    event::{DomainEvent, EventEnvelope, EventObjectRef, EventSource, EventVisibility},
    memory::{parse_memory_kind, serialize_memory_kind, MemoryRecord},
    model::{
        Artifact, ChannelObject, Session, Subagent, SubagentStatus, Task, TaskHost, TaskKind,
        TaskPriority, WorkItem,
    },
    runtime_log::{parse_log_level, serialize_log_level, RuntimeLogEntry},
    scheduler::{QueuedTask, ScheduledWakeup},
    state::TaskWaitKind,
    policy::ApprovalRiskLevel,
};

const META_FILE: &str = "runtime.meta";
const SESSIONS_FILE: &str = "sessions.tsv";
const TASKS_FILE: &str = "tasks.tsv";
const EVENTS_FILE: &str = "events.tsv";
const QUEUE_FILE: &str = "queue.tsv";
const WAKEUPS_FILE: &str = "wakeups.tsv";
const MEMORY_FILE: &str = "memory.tsv";
const LOGS_FILE: &str = "logs.tsv";
const ARTIFACTS_FILE: &str = "artifacts.tsv";
const SUBAGENTS_FILE: &str = "subagents.tsv";
const CHANNEL_OBJECTS_FILE: &str = "channel_objects.tsv";
const WORK_ITEMS_FILE: &str = "work_items.tsv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStateStore {
    root: PathBuf,
}

impl RuntimeStateStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn meta_path(&self) -> PathBuf {
        self.root.join(META_FILE)
    }

    pub fn sessions_path(&self) -> PathBuf {
        self.root.join(SESSIONS_FILE)
    }

    pub fn tasks_path(&self) -> PathBuf {
        self.root.join(TASKS_FILE)
    }

    pub fn events_path(&self) -> PathBuf {
        self.root.join(EVENTS_FILE)
    }

    pub fn queue_path(&self) -> PathBuf {
        self.root.join(QUEUE_FILE)
    }

    pub fn wakeups_path(&self) -> PathBuf {
        self.root.join(WAKEUPS_FILE)
    }

    pub fn memory_path(&self) -> PathBuf {
        self.root.join(MEMORY_FILE)
    }

    pub fn logs_path(&self) -> PathBuf {
        self.root.join(LOGS_FILE)
    }

    pub fn artifacts_path(&self) -> PathBuf {
        self.root.join(ARTIFACTS_FILE)
    }

    pub fn subagents_path(&self) -> PathBuf {
        self.root.join(SUBAGENTS_FILE)
    }

    pub fn channel_objects_path(&self) -> PathBuf {
        self.root.join(CHANNEL_OBJECTS_FILE)
    }

    pub fn work_items_path(&self) -> PathBuf {
        self.root.join(WORK_ITEMS_FILE)
    }

    pub fn save(
        &self,
        workspace_root: Option<&Path>,
        next_sequence: u64,
        sessions: &HashMap<String, Session>,
        tasks: &HashMap<String, Task>,
        artifacts: &HashMap<String, Artifact>,
        subagents: &HashMap<String, Subagent>,
        channel_objects: &HashMap<String, ChannelObject>,
        work_items: &HashMap<String, WorkItem>,
        events: &[EventEnvelope],
        logs: &[RuntimeLogEntry],
        queued_tasks: &[QueuedTask],
        wakeups: &[ScheduledWakeup],
        memory_records: &[MemoryRecord],
    ) -> Result<(), PersistenceError> {
        fs::create_dir_all(&self.root)?;
        fs::write(
            self.meta_path(),
            serialize_meta(workspace_root, next_sequence),
        )?;

        let mut sessions_list = sessions.values().cloned().collect::<Vec<_>>();
        sessions_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(self.sessions_path(), serialize_sessions(&sessions_list))?;

        let mut tasks_list = tasks.values().cloned().collect::<Vec<_>>();
        tasks_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(self.tasks_path(), serialize_tasks(&tasks_list))?;

        let mut artifacts_list = artifacts.values().cloned().collect::<Vec<_>>();
        artifacts_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(self.artifacts_path(), serialize_artifacts(&artifacts_list))?;

        let mut subagents_list = subagents.values().cloned().collect::<Vec<_>>();
        subagents_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(self.subagents_path(), serialize_subagents(&subagents_list))?;

        let mut channel_objects_list = channel_objects.values().cloned().collect::<Vec<_>>();
        channel_objects_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(
            self.channel_objects_path(),
            serialize_channel_objects(&channel_objects_list),
        )?;

        let mut work_items_list = work_items.values().cloned().collect::<Vec<_>>();
        work_items_list.sort_by(|a, b| a.id.cmp(&b.id));
        fs::write(self.work_items_path(), serialize_work_items(&work_items_list))?;

        fs::write(self.events_path(), serialize_events(events))?;
        fs::write(self.logs_path(), serialize_logs(logs))?;
        fs::write(self.queue_path(), serialize_queue(queued_tasks))?;
        fs::write(self.wakeups_path(), serialize_wakeups(wakeups))?;
        fs::write(self.memory_path(), serialize_memory(memory_records))?;
        Ok(())
    }

    pub fn load(
        &self,
    ) -> Result<
        (
            Option<PathBuf>,
            u64,
            HashMap<String, Session>,
            HashMap<String, Task>,
            HashMap<String, Artifact>,
            HashMap<String, Subagent>,
            HashMap<String, ChannelObject>,
            HashMap<String, WorkItem>,
            Vec<EventEnvelope>,
            Vec<RuntimeLogEntry>,
            Vec<QueuedTask>,
            Vec<ScheduledWakeup>,
            Vec<MemoryRecord>,
        ),
        PersistenceError,
    > {
        let meta = parse_meta(&fs::read_to_string(self.meta_path())?)?;
        let sessions = parse_sessions(&fs::read_to_string(self.sessions_path())?)?;
        let tasks = parse_tasks(&fs::read_to_string(self.tasks_path())?)?;
        let artifacts = parse_artifacts(&fs::read_to_string(self.artifacts_path())?)?;
        let subagents = parse_subagents(&fs::read_to_string(self.subagents_path())?)?;
        let channel_objects =
            parse_channel_objects(&fs::read_to_string(self.channel_objects_path())?)?;
        let work_items = parse_work_items(&fs::read_to_string(self.work_items_path())?)?;
        let events = parse_events(&fs::read_to_string(self.events_path())?)?;
        let logs = parse_logs(&fs::read_to_string(self.logs_path())?)?;
        let queued_tasks = parse_queue(&fs::read_to_string(self.queue_path())?)?;
        let wakeups = parse_wakeups(&fs::read_to_string(self.wakeups_path())?)?;
        let memory_records = parse_memory(&fs::read_to_string(self.memory_path())?)?;

        Ok((
            meta.workspace_root,
            meta.next_sequence,
            sessions.into_iter().map(|value| (value.id.clone(), value)).collect(),
            tasks.into_iter().map(|value| (value.id.clone(), value)).collect(),
            artifacts
                .into_iter()
                .map(|value| (value.id.clone(), value))
                .collect(),
            subagents
                .into_iter()
                .map(|value| (value.id.clone(), value))
                .collect(),
            channel_objects
                .into_iter()
                .map(|value| (value.id.clone(), value))
                .collect(),
            work_items
                .into_iter()
                .map(|value| (value.id.clone(), value))
                .collect(),
            events,
            logs,
            queued_tasks,
            wakeups,
            memory_records,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    Io(String),
    InvalidFormat(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "{message}"),
            Self::InvalidFormat(message) => write!(f, "{message}"),
        }
    }
}

impl Error for PersistenceError {}

impl From<std::io::Error> for PersistenceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Debug)]
struct RuntimeMeta {
    workspace_root: Option<PathBuf>,
    next_sequence: u64,
}

fn serialize_meta(workspace_root: Option<&Path>, next_sequence: u64) -> String {
    let workspace = workspace_root
        .map(|path| escape_field(&path.to_string_lossy()))
        .unwrap_or_default();
    format!("workspace_root\t{workspace}\nnext_sequence\t{next_sequence}\n")
}

fn parse_meta(source: &str) -> Result<RuntimeMeta, PersistenceError> {
    let mut workspace_root = None;
    let mut next_sequence = 0_u64;

    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 2 {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid meta line `{line}`"
            )));
        }
        match fields[0].as_str() {
            "workspace_root" => {
                if !fields[1].is_empty() {
                    workspace_root = Some(PathBuf::from(&fields[1]));
                }
            }
            "next_sequence" => {
                next_sequence = fields[1].parse::<u64>().map_err(|_| {
                    PersistenceError::InvalidFormat("invalid next_sequence value".into())
                })?;
            }
            key => {
                return Err(PersistenceError::InvalidFormat(format!(
                    "unknown meta key `{key}`"
                )));
            }
        }
    }

    Ok(RuntimeMeta {
        workspace_root,
        next_sequence,
    })
}

fn serialize_sessions(sessions: &[Session]) -> String {
    sessions
        .iter()
        .map(|session| {
            join_fields(&[
                "session".into(),
                session.id.clone(),
                session.title.clone(),
                session.workspace_root.clone(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_sessions(source: &str) -> Result<Vec<Session>, PersistenceError> {
    let mut sessions = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 4 || fields[0] != "session" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid session line `{line}`"
            )));
        }
        sessions.push(Session {
            id: fields[1].clone(),
            title: fields[2].clone(),
            workspace_root: fields[3].clone(),
        });
    }
    Ok(sessions)
}

fn serialize_tasks(tasks: &[Task]) -> String {
    tasks.iter()
        .map(|task| {
            let (host_kind, host_system, host_object_type, host_object_id) = match &task.host {
                TaskHost::Internal => (
                    "internal".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                TaskHost::External {
                    system,
                    object_type,
                    object_id,
                } => (
                    "external".to_string(),
                    system.clone(),
                    object_type.clone(),
                    object_id.clone(),
                ),
            };

            join_fields(&[
                "task".into(),
                task.id.clone(),
                task.session_id.clone(),
                task.work_item_id.clone().unwrap_or_default(),
                task.title.clone(),
                serialize_task_kind(&task.kind),
                host_kind,
                host_system,
                host_object_type,
                host_object_id,
                serialize_task_priority(&task.priority),
                task.created_at_ms.to_string(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn serialize_channel_objects(items: &[ChannelObject]) -> String {
    items.iter()
        .map(|item| {
            join_fields(&[
                "channel_object".into(),
                item.id.clone(),
                item.source_system.clone(),
                item.object_type.clone(),
                item.external_id.clone(),
                item.workspace_id.clone().unwrap_or_default(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn serialize_artifacts(items: &[Artifact]) -> String {
    items.iter()
        .map(|item| {
            join_fields(&[
                "artifact".into(),
                item.id.clone(),
                item.task_id.clone(),
                item.name.clone(),
                item.path.clone(),
                item.expires_at_ms
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn serialize_logs(items: &[RuntimeLogEntry]) -> String {
    items.iter()
        .map(|item| {
            join_fields(&[
                "log".into(),
                item.id.clone(),
                serialize_log_level(&item.level),
                item.message.clone(),
                item.task_id.clone().unwrap_or_default(),
                item.timestamp_ms.to_string(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn serialize_subagents(items: &[Subagent]) -> String {
    items.iter()
        .map(|item| {
            join_fields(&[
                "subagent".into(),
                item.id.clone(),
                item.parent_task_id.clone(),
                item.role.clone(),
                serialize_subagent_status(&item.status),
                item.detail.clone().unwrap_or_default(),
                item.background.to_string(),
                item.created_at_ms.to_string(),
                item.updated_at_ms.to_string(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_subagents(source: &str) -> Result<Vec<Subagent>, PersistenceError> {
    let mut values = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 9 || fields[0] != "subagent" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid subagent line `{line}`"
            )));
        }
        values.push(Subagent {
            id: fields[1].clone(),
            parent_task_id: fields[2].clone(),
            role: fields[3].clone(),
            status: parse_subagent_status(&fields[4])?,
            detail: optional_field(&fields[5]),
            background: parse_bool(&fields[6], "subagent.background")?,
            created_at_ms: parse_u64(&fields[7], "subagent.created_at_ms")?,
            updated_at_ms: parse_u64(&fields[8], "subagent.updated_at_ms")?,
        });
    }
    Ok(values)
}

fn parse_logs(source: &str) -> Result<Vec<RuntimeLogEntry>, PersistenceError> {
    let mut values = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 6 || fields[0] != "log" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid log line `{line}`"
            )));
        }
        values.push(RuntimeLogEntry {
            id: fields[1].clone(),
            level: parse_log_level(&fields[2]).ok_or_else(|| {
                PersistenceError::InvalidFormat(format!("invalid log level `{}`", fields[2]))
            })?,
            message: fields[3].clone(),
            task_id: optional_field(&fields[4]),
            timestamp_ms: parse_u64(&fields[5], "log.timestamp_ms")?,
        });
    }
    Ok(values)
}

fn parse_artifacts(source: &str) -> Result<Vec<Artifact>, PersistenceError> {
    let mut values = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 6 || fields[0] != "artifact" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid artifact line `{line}`"
            )));
        }
        values.push(Artifact {
            id: fields[1].clone(),
            task_id: fields[2].clone(),
            name: fields[3].clone(),
            path: fields[4].clone(),
            expires_at_ms: if fields[5].is_empty() {
                None
            } else {
                Some(parse_u64(&fields[5], "artifact.expires_at_ms")?)
            },
        });
    }
    Ok(values)
}

fn parse_channel_objects(source: &str) -> Result<Vec<ChannelObject>, PersistenceError> {
    let mut values = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 6 || fields[0] != "channel_object" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid channel object line `{line}`"
            )));
        }
        values.push(ChannelObject {
            id: fields[1].clone(),
            source_system: fields[2].clone(),
            object_type: fields[3].clone(),
            external_id: fields[4].clone(),
            workspace_id: optional_field(&fields[5]),
        });
    }
    Ok(values)
}

fn serialize_work_items(items: &[WorkItem]) -> String {
    items.iter()
        .map(|item| {
            join_fields(&[
                "work_item".into(),
                item.id.clone(),
                item.source_system.clone(),
                item.summary.clone(),
                item.channel_object_id.clone().unwrap_or_default(),
                item.status.clone(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_work_items(source: &str) -> Result<Vec<WorkItem>, PersistenceError> {
    let mut values = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 6 || fields[0] != "work_item" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid work item line `{line}`"
            )));
        }
        values.push(WorkItem {
            id: fields[1].clone(),
            source_system: fields[2].clone(),
            summary: fields[3].clone(),
            channel_object_id: optional_field(&fields[4]),
            status: fields[5].clone(),
        });
    }
    Ok(values)
}

fn parse_tasks(source: &str) -> Result<Vec<Task>, PersistenceError> {
    let mut tasks = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 12 || fields[0] != "task" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid task line `{line}`"
            )));
        }
        let host = match fields[6].as_str() {
            "internal" => TaskHost::Internal,
            "external" => TaskHost::External {
                system: fields[7].clone(),
                object_type: fields[8].clone(),
                object_id: fields[9].clone(),
            },
            other => {
                return Err(PersistenceError::InvalidFormat(format!(
                    "unsupported task host `{other}`"
                )))
            }
        };
        tasks.push(Task {
            id: fields[1].clone(),
            session_id: fields[2].clone(),
            work_item_id: optional_field(&fields[3]),
            title: fields[4].clone(),
            kind: parse_task_kind(&fields[5])?,
            host,
            priority: parse_task_priority(&fields[10])?,
            created_at_ms: parse_u64(&fields[11], "task.created_at_ms")?,
        });
    }
    Ok(tasks)
}

fn serialize_events(events: &[EventEnvelope]) -> String {
    events
        .iter()
        .map(serialize_event)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_events(source: &str) -> Result<Vec<EventEnvelope>, PersistenceError> {
    let mut events = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        events.push(parse_event(line)?);
    }
    Ok(events)
}

fn serialize_queue(entries: &[QueuedTask]) -> String {
    entries
        .iter()
        .map(|entry| {
            join_fields(&[
                "queue".into(),
                entry.task_id.clone(),
                serialize_task_priority(&entry.priority),
                entry.enqueued_at_ms.to_string(),
                entry.background.to_string(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_queue(source: &str) -> Result<Vec<QueuedTask>, PersistenceError> {
    let mut entries = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 5 || fields[0] != "queue" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid queue line `{line}`"
            )));
        }
        entries.push(QueuedTask {
            task_id: fields[1].clone(),
            priority: parse_task_priority(&fields[2])?,
            enqueued_at_ms: parse_u64(&fields[3], "queue.enqueued_at_ms")?,
            background: parse_bool(&fields[4], "queue.background")?,
        });
    }
    Ok(entries)
}

fn serialize_wakeups(entries: &[ScheduledWakeup]) -> String {
    entries
        .iter()
        .map(|entry| {
            join_fields(&[
                "wakeup".into(),
                entry.task_id.clone(),
                entry.wake_at_ms.to_string(),
                entry.reason.clone(),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_wakeups(source: &str) -> Result<Vec<ScheduledWakeup>, PersistenceError> {
    let mut entries = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 4 || fields[0] != "wakeup" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid wakeup line `{line}`"
            )));
        }
        entries.push(ScheduledWakeup {
            task_id: fields[1].clone(),
            wake_at_ms: parse_u64(&fields[2], "wakeup.wake_at_ms")?,
            reason: fields[3].clone(),
        });
    }
    Ok(entries)
}

fn serialize_memory(records: &[MemoryRecord]) -> String {
    records
        .iter()
        .map(|record| {
            join_fields(&[
                "memory".into(),
                record.id.clone(),
                serialize_memory_kind(&record.kind),
                record.scope.clone(),
                record.subject_ref.clone().unwrap_or_default(),
                record.content.clone(),
                record.source.clone(),
                record.confidence.to_string(),
                record.updated_at_ms.to_string(),
                record.supersedes.clone().unwrap_or_default(),
                join_subfields(&record.tags),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_memory(source: &str) -> Result<Vec<MemoryRecord>, PersistenceError> {
    let mut records = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_fields(line)?;
        if fields.len() != 11 || fields[0] != "memory" {
            return Err(PersistenceError::InvalidFormat(format!(
                "invalid memory line `{line}`"
            )));
        }
        let kind = parse_memory_kind(&fields[2]).ok_or_else(|| {
            PersistenceError::InvalidFormat(format!("unsupported memory kind `{}`", fields[2]))
        })?;
        records.push(MemoryRecord {
            id: fields[1].clone(),
            kind,
            scope: fields[3].clone(),
            subject_ref: optional_field(&fields[4]),
            content: fields[5].clone(),
            source: fields[6].clone(),
            confidence: parse_u8(&fields[7], "memory.confidence")?,
            updated_at_ms: parse_u64(&fields[8], "memory.updated_at_ms")?,
            supersedes: optional_field(&fields[9]),
            tags: split_subfields(&fields[10])?,
        });
    }
    Ok(records)
}

fn serialize_event(event: &EventEnvelope) -> String {
    let (source_kind, source_value) = match &event.source {
        EventSource::Runtime => ("runtime".to_string(), String::new()),
        EventSource::Connector(value) => ("connector".to_string(), value.clone()),
        EventSource::Tool(value) => ("tool".to_string(), value.clone()),
        EventSource::Model(value) => ("model".to_string(), value.clone()),
        EventSource::User => ("user".to_string(), String::new()),
        EventSource::Scheduler => ("scheduler".to_string(), String::new()),
    };
    let (event_kind, arg1, arg2, arg3, arg4) = serialize_domain_event(&event.event);

    join_fields(&[
        "event".into(),
        event.event_id.clone(),
        event.object.object_type.clone(),
        event.object.object_id.clone(),
        event.task_id.clone().unwrap_or_default(),
        event.session_id.clone().unwrap_or_default(),
        source_kind,
        source_value,
        event.timestamp_ms.to_string(),
        event.sequence.to_string(),
        serialize_visibility(&event.visibility),
        event_kind,
        arg1,
        arg2,
        arg3,
        arg4,
    ])
}

fn parse_event(line: &str) -> Result<EventEnvelope, PersistenceError> {
    let fields = split_fields(line)?;
    if fields.len() != 16 || fields[0] != "event" {
        return Err(PersistenceError::InvalidFormat(format!(
            "invalid event line `{line}`"
        )));
    }
    Ok(EventEnvelope {
        event_id: fields[1].clone(),
        object: EventObjectRef {
            object_type: fields[2].clone(),
            object_id: fields[3].clone(),
        },
        task_id: optional_field(&fields[4]),
        session_id: optional_field(&fields[5]),
        source: parse_source(&fields[6], &fields[7])?,
        timestamp_ms: parse_u64(&fields[8], "event.timestamp_ms")?,
        sequence: parse_u64(&fields[9], "event.sequence")?,
        visibility: parse_visibility(&fields[10])?,
        event: parse_domain_event(
            &fields[11],
            &fields[12],
            &fields[13],
            &fields[14],
            &fields[15],
        )?,
    })
}

fn serialize_domain_event(event: &DomainEvent) -> (String, String, String, String, String) {
    match event {
        DomainEvent::TaskCreated { initial_phase, host } => {
            let (host_kind, host_system, host_object_type, host_object_id) = match host {
                TaskHost::Internal => (
                    "internal".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                TaskHost::External {
                    system,
                    object_type,
                    object_id,
                } => (
                    "external".to_string(),
                    system.clone(),
                    object_type.clone(),
                    object_id.clone(),
                ),
            };
            (
                "task_created".into(),
                initial_phase.clone().unwrap_or_default(),
                host_kind,
                host_system,
                join_subfields(&[host_object_type, host_object_id]),
            )
        }
        DomainEvent::TaskStarted => ("task_started".into(), String::new(), String::new(), String::new(), String::new()),
        DomainEvent::TaskWorkflowModeChanged { mode, reason } => (
            "task_workflow_mode_changed".into(),
            serialize_workflow_mode(mode),
            reason.clone(),
            String::new(),
            String::new(),
        ),
        DomainEvent::TaskProgress {
            phase,
            current_step,
            total_steps,
            progress_text,
        } => (
            "task_progress".into(),
            phase.clone().unwrap_or_default(),
            current_step.clone().unwrap_or_default(),
            total_steps.map(|value| value.to_string()).unwrap_or_default(),
            progress_text.clone(),
        ),
        DomainEvent::TaskHeartbeat { message } => (
            "task_heartbeat".into(),
            message.clone().unwrap_or_default(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::TaskWaiting {
            kind,
            reason,
            resume_checkpoint,
        } => (
            "task_waiting".into(),
            serialize_wait_kind(kind),
            reason.clone(),
            resume_checkpoint.clone().unwrap_or_default(),
            String::new(),
        ),
        DomainEvent::TaskBlocked { reason } => (
            "task_blocked".into(),
            reason.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::TaskResumed => ("task_resumed".into(), String::new(), String::new(), String::new(), String::new()),
        DomainEvent::TaskCompleted { summary } => (
            "task_completed".into(),
            summary.clone().unwrap_or_default(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::TaskFailed { error } => (
            "task_failed".into(),
            error.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::TaskCancelled => ("task_cancelled".into(), String::new(), String::new(), String::new(), String::new()),
        DomainEvent::ToolCalled { name } => (
            "tool_called".into(),
            name.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::ToolFinished { name } => (
            "tool_finished".into(),
            name.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::SubagentSpawned {
            subagent_id,
            role,
            background,
        } => (
            "subagent_spawned".into(),
            subagent_id.clone(),
            role.clone(),
            background.to_string(),
            String::new(),
        ),
        DomainEvent::SubagentUpdated {
            subagent_id,
            status,
            detail,
        } => (
            "subagent_updated".into(),
            subagent_id.clone(),
            serialize_subagent_status(status),
            detail.clone().unwrap_or_default(),
            String::new(),
        ),
        DomainEvent::PolicyAllowed {
            action,
            detail,
            risk_level,
        } => (
            "policy_allowed".into(),
            action.clone(),
            detail.clone(),
            serialize_approval_risk_level(risk_level),
            String::new(),
        ),
        DomainEvent::PolicyDenied {
            action,
            reason,
            risk_level,
        } => (
            "policy_denied".into(),
            action.clone(),
            reason.clone(),
            serialize_approval_risk_level(risk_level),
            String::new(),
        ),
        DomainEvent::PolicyApprovalRequired {
            action,
            reason,
            risk_level,
        } => (
            "policy_approval_required".into(),
            action.clone(),
            reason.clone(),
            serialize_approval_risk_level(risk_level),
            String::new(),
        ),
        DomainEvent::PolicyApproved {
            action,
            checkpoint,
            risk_level,
            decision_source,
            resolved_by,
        } => (
            "policy_approved".into(),
            action.clone(),
            checkpoint.clone(),
            serialize_approval_risk_level(risk_level),
            join_subfields(&[decision_source.clone(), resolved_by.clone()]),
        ),
        DomainEvent::PolicyRejected {
            action,
            checkpoint,
            reason,
            risk_level,
            decision_source,
            resolved_by,
        } => (
            "policy_rejected".into(),
            action.clone(),
            checkpoint.clone(),
            serialize_approval_risk_level(risk_level),
            join_subfields(&[reason.clone(), decision_source.clone(), resolved_by.clone()]),
        ),
        DomainEvent::ExternalReceived { connector_id } => (
            "external_received".into(),
            connector_id.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        DomainEvent::ExternalSynced { connector_id } => (
            "external_synced".into(),
            connector_id.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
    }
}

fn parse_domain_event(
    kind: &str,
    arg1: &str,
    arg2: &str,
    arg3: &str,
    arg4: &str,
) -> Result<DomainEvent, PersistenceError> {
    match kind {
        "task_created" => {
            let extra = split_subfields(arg4)?;
            let host = match arg2 {
                "internal" => TaskHost::Internal,
                "external" => TaskHost::External {
                    system: arg3.to_string(),
                    object_type: extra.first().cloned().unwrap_or_default(),
                    object_id: extra.get(1).cloned().unwrap_or_default(),
                },
                other => {
                    return Err(PersistenceError::InvalidFormat(format!(
                        "unsupported task_created host `{other}`"
                    )))
                }
            };
            Ok(DomainEvent::TaskCreated {
                initial_phase: optional_field(arg1),
                host,
            })
        }
        "task_started" => Ok(DomainEvent::TaskStarted),
        "task_workflow_mode_changed" => Ok(DomainEvent::TaskWorkflowModeChanged {
            mode: parse_workflow_mode(arg1)?,
            reason: arg2.to_string(),
        }),
        "task_progress" => Ok(DomainEvent::TaskProgress {
            phase: optional_field(arg1),
            current_step: optional_field(arg2),
            total_steps: if arg3.is_empty() {
                None
            } else {
                Some(parse_u32(arg3, "task_progress.total_steps")?)
            },
            progress_text: arg4.to_string(),
        }),
        "task_heartbeat" => Ok(DomainEvent::TaskHeartbeat {
            message: optional_field(arg1),
        }),
        "task_waiting" => Ok(DomainEvent::TaskWaiting {
            kind: parse_wait_kind(arg1)?,
            reason: arg2.to_string(),
            resume_checkpoint: optional_field(arg3),
        }),
        "task_blocked" => Ok(DomainEvent::TaskBlocked {
            reason: arg1.to_string(),
        }),
        "task_resumed" => Ok(DomainEvent::TaskResumed),
        "task_completed" => Ok(DomainEvent::TaskCompleted {
            summary: optional_field(arg1),
        }),
        "task_failed" => Ok(DomainEvent::TaskFailed {
            error: arg1.to_string(),
        }),
        "task_cancelled" => Ok(DomainEvent::TaskCancelled),
        "tool_called" => Ok(DomainEvent::ToolCalled {
            name: arg1.to_string(),
        }),
        "tool_finished" => Ok(DomainEvent::ToolFinished {
            name: arg1.to_string(),
        }),
        "subagent_spawned" => Ok(DomainEvent::SubagentSpawned {
            subagent_id: arg1.to_string(),
            role: arg2.to_string(),
            background: parse_bool(arg3, "subagent_spawned.background")?,
        }),
        "subagent_updated" => Ok(DomainEvent::SubagentUpdated {
            subagent_id: arg1.to_string(),
            status: parse_subagent_status(arg2)?,
            detail: optional_field(arg3),
        }),
        "policy_allowed" => Ok(DomainEvent::PolicyAllowed {
            action: arg1.to_string(),
            detail: arg2.to_string(),
            risk_level: parse_approval_risk_level(arg3)?,
        }),
        "policy_denied" => Ok(DomainEvent::PolicyDenied {
            action: arg1.to_string(),
            reason: arg2.to_string(),
            risk_level: parse_approval_risk_level(arg3)?,
        }),
        "policy_approval_required" => Ok(DomainEvent::PolicyApprovalRequired {
            action: arg1.to_string(),
            reason: arg2.to_string(),
            risk_level: parse_approval_risk_level(arg3)?,
        }),
        "policy_approved" => Ok(DomainEvent::PolicyApproved {
            action: arg1.to_string(),
            checkpoint: arg2.to_string(),
            risk_level: parse_approval_risk_level(arg3)?,
            decision_source: split_subfields(arg4)?.first().cloned().unwrap_or_default(),
            resolved_by: split_subfields(arg4)?.get(1).cloned().unwrap_or_default(),
        }),
        "policy_rejected" => Ok(DomainEvent::PolicyRejected {
            action: arg1.to_string(),
            checkpoint: arg2.to_string(),
            risk_level: parse_approval_risk_level(arg3)?,
            reason: split_subfields(arg4)?.first().cloned().unwrap_or_default(),
            decision_source: split_subfields(arg4)?.get(1).cloned().unwrap_or_default(),
            resolved_by: split_subfields(arg4)?.get(2).cloned().unwrap_or_default(),
        }),
        "external_received" => Ok(DomainEvent::ExternalReceived {
            connector_id: arg1.to_string(),
        }),
        "external_synced" => Ok(DomainEvent::ExternalSynced {
            connector_id: arg1.to_string(),
        }),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported domain event `{other}`"
        ))),
    }
}

fn serialize_task_kind(value: &TaskKind) -> String {
    match value {
        TaskKind::Internal => "internal".into(),
        TaskKind::ExternalHost => "external_host".into(),
    }
}

fn parse_task_kind(value: &str) -> Result<TaskKind, PersistenceError> {
    match value {
        "internal" => Ok(TaskKind::Internal),
        "external_host" => Ok(TaskKind::ExternalHost),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported task kind `{other}`"
        ))),
    }
}

fn serialize_task_priority(value: &TaskPriority) -> String {
    match value {
        TaskPriority::Low => "low".into(),
        TaskPriority::Normal => "normal".into(),
        TaskPriority::High => "high".into(),
    }
}

fn parse_task_priority(value: &str) -> Result<TaskPriority, PersistenceError> {
    match value {
        "low" => Ok(TaskPriority::Low),
        "normal" => Ok(TaskPriority::Normal),
        "high" => Ok(TaskPriority::High),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported task priority `{other}`"
        ))),
    }
}

fn serialize_visibility(value: &EventVisibility) -> String {
    match value {
        EventVisibility::Internal => "internal".into(),
        EventVisibility::Ui => "ui".into(),
        EventVisibility::Audit => "audit".into(),
        EventVisibility::ExternalSync => "external_sync".into(),
    }
}

fn parse_visibility(value: &str) -> Result<EventVisibility, PersistenceError> {
    match value {
        "internal" => Ok(EventVisibility::Internal),
        "ui" => Ok(EventVisibility::Ui),
        "audit" => Ok(EventVisibility::Audit),
        "external_sync" => Ok(EventVisibility::ExternalSync),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported visibility `{other}`"
        ))),
    }
}

fn parse_source(kind: &str, value: &str) -> Result<EventSource, PersistenceError> {
    match kind {
        "runtime" => Ok(EventSource::Runtime),
        "connector" => Ok(EventSource::Connector(value.to_string())),
        "tool" => Ok(EventSource::Tool(value.to_string())),
        "model" => Ok(EventSource::Model(value.to_string())),
        "user" => Ok(EventSource::User),
        "scheduler" => Ok(EventSource::Scheduler),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported event source `{other}`"
        ))),
    }
}

fn serialize_wait_kind(value: &TaskWaitKind) -> String {
    match value {
        TaskWaitKind::User => "user".into(),
        TaskWaitKind::Approval => "approval".into(),
        TaskWaitKind::Tool => "tool".into(),
        TaskWaitKind::Schedule => "schedule".into(),
        TaskWaitKind::Subtask => "subtask".into(),
    }
}

fn parse_wait_kind(value: &str) -> Result<TaskWaitKind, PersistenceError> {
    match value {
        "user" => Ok(TaskWaitKind::User),
        "approval" => Ok(TaskWaitKind::Approval),
        "tool" => Ok(TaskWaitKind::Tool),
        "schedule" => Ok(TaskWaitKind::Schedule),
        "subtask" => Ok(TaskWaitKind::Subtask),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported wait kind `{other}`"
        ))),
    }
}

fn serialize_workflow_mode(value: &crate::state::TaskWorkflowMode) -> String {
    match value {
        crate::state::TaskWorkflowMode::Simple => "simple".into(),
        crate::state::TaskWorkflowMode::Complex => "complex".into(),
    }
}

fn parse_workflow_mode(
    value: &str,
) -> Result<crate::state::TaskWorkflowMode, PersistenceError> {
    match value {
        "simple" => Ok(crate::state::TaskWorkflowMode::Simple),
        "complex" => Ok(crate::state::TaskWorkflowMode::Complex),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported workflow mode `{other}`"
        ))),
    }
}

fn serialize_approval_risk_level(value: &ApprovalRiskLevel) -> String {
    match value {
        ApprovalRiskLevel::Low => "low".into(),
        ApprovalRiskLevel::High => "high".into(),
    }
}

fn parse_approval_risk_level(value: &str) -> Result<ApprovalRiskLevel, PersistenceError> {
    match value {
        "low" => Ok(ApprovalRiskLevel::Low),
        "high" => Ok(ApprovalRiskLevel::High),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported approval risk `{other}`"
        ))),
    }
}

fn serialize_subagent_status(value: &SubagentStatus) -> String {
    match value {
        SubagentStatus::Queued => "queued".into(),
        SubagentStatus::Running => "running".into(),
        SubagentStatus::Waiting => "waiting".into(),
        SubagentStatus::Done => "done".into(),
        SubagentStatus::Failed => "failed".into(),
    }
}

fn parse_subagent_status(value: &str) -> Result<SubagentStatus, PersistenceError> {
    match value {
        "queued" => Ok(SubagentStatus::Queued),
        "running" => Ok(SubagentStatus::Running),
        "waiting" => Ok(SubagentStatus::Waiting),
        "done" => Ok(SubagentStatus::Done),
        "failed" => Ok(SubagentStatus::Failed),
        other => Err(PersistenceError::InvalidFormat(format!(
            "unsupported subagent status `{other}`"
        ))),
    }
}

fn parse_u64(value: &str, field: &str) -> Result<u64, PersistenceError> {
    value.parse::<u64>()
        .map_err(|_| PersistenceError::InvalidFormat(format!("invalid integer for `{field}`")))
}

fn parse_u32(value: &str, field: &str) -> Result<u32, PersistenceError> {
    value.parse::<u32>()
        .map_err(|_| PersistenceError::InvalidFormat(format!("invalid integer for `{field}`")))
}

fn parse_u8(value: &str, field: &str) -> Result<u8, PersistenceError> {
    value.parse::<u8>()
        .map_err(|_| PersistenceError::InvalidFormat(format!("invalid integer for `{field}`")))
}

fn parse_bool(value: &str, field: &str) -> Result<bool, PersistenceError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(PersistenceError::InvalidFormat(format!(
            "invalid boolean for `{field}`"
        ))),
    }
}

fn join_fields(fields: &[String]) -> String {
    fields
        .iter()
        .map(|value| escape_field(value))
        .collect::<Vec<_>>()
        .join("\t")
}

fn split_fields(line: &str) -> Result<Vec<String>, PersistenceError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            let decoded = match ch {
                'n' => '\n',
                't' => '\t',
                '\\' => '\\',
                'p' => '|',
                other => {
                    return Err(PersistenceError::InvalidFormat(format!(
                        "unsupported escape sequence `\\{other}`"
                    )))
                }
            };
            current.push(decoded);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '\t' => {
                fields.push(current);
                current = String::new();
            }
            other => current.push(other),
        }
    }
    if escaped {
        return Err(PersistenceError::InvalidFormat(
            "dangling escape sequence".into(),
        ));
    }
    fields.push(current);
    Ok(fields)
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('|', "\\p")
}

fn join_subfields(fields: &[String]) -> String {
    fields
        .iter()
        .map(|value| escape_field(value))
        .collect::<Vec<_>>()
        .join("|")
}

fn split_subfields(value: &str) -> Result<Vec<String>, PersistenceError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            let decoded = match ch {
                'n' => '\n',
                't' => '\t',
                '\\' => '\\',
                'p' => '|',
                other => {
                    return Err(PersistenceError::InvalidFormat(format!(
                        "unsupported subfield escape `\\{other}`"
                    )))
                }
            };
            current.push(decoded);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '|' => {
                fields.push(current);
                current = String::new();
            }
            other => current.push(other),
        }
    }
    if escaped {
        return Err(PersistenceError::InvalidFormat(
            "dangling subfield escape sequence".into(),
        ));
    }
    fields.push(current);
    Ok(fields)
}

fn optional_field(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        connector::ConnectorSample,
        event::{DomainEvent, EventEnvelope, EventSource, EventVisibility},
        memory::{MemoryKind, MemoryRecord},
        model::{
            Artifact, ChannelObject, Session, Subagent, SubagentStatus, Task, TaskHost, TaskKind,
            TaskPriority, WorkItem,
        },
        runtime_log::RuntimeLogEntry,
        scheduler::{QueuedTask, ScheduledWakeup},
    };

    use super::RuntimeStateStore;

    #[test]
    fn store_round_trips_sessions_tasks_and_events() {
        let root = temp_path("store-roundtrip");
        let store = RuntimeStateStore::new(&root);
        let mut sessions = HashMap::new();
        sessions.insert(
            "session-1".into(),
            Session {
                id: "session-1".into(),
                title: "Demo".into(),
                workspace_root: "/tmp/agentboard".into(),
            },
        );
        let mut tasks = HashMap::new();
        tasks.insert(
            "task-1".into(),
            Task {
                id: "task-1".into(),
                session_id: "session-1".into(),
                work_item_id: None,
                title: "Persist".into(),
                kind: TaskKind::Internal,
                host: TaskHost::Internal,
                priority: TaskPriority::Normal,
                created_at_ms: 1_000,
            },
        );
        let mut channel_objects = HashMap::new();
        channel_objects.insert(
            "object-1".into(),
            ChannelObject {
                id: "object-1".into(),
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
            },
        );
        let mut work_items = HashMap::new();
        work_items.insert(
            "work-1".into(),
            WorkItem {
                id: "work-1".into(),
                source_system: "feishu".into(),
                summary: "Prepare report".into(),
                channel_object_id: Some("object-1".into()),
                status: "open".into(),
            },
        );
        let mut artifacts = HashMap::new();
        artifacts.insert(
            "artifact-1".into(),
            Artifact {
                id: "artifact-1".into(),
                task_id: "task-1".into(),
                name: "report.md".into(),
                path: "/tmp/agentboard/.taskloop/tmp/artifact-1-report.md".into(),
                expires_at_ms: Some(86_401_000),
            },
        );
        let mut subagents = HashMap::new();
        subagents.insert(
            "sub-1".into(),
            Subagent {
                id: "sub-1".into(),
                parent_task_id: "task-1".into(),
                role: "reviewer".into(),
                status: SubagentStatus::Running,
                detail: Some("reviewing".into()),
                background: true,
                created_at_ms: 1_600,
                updated_at_ms: 1_700,
            },
        );
        let events = vec![EventEnvelope::for_task(
            "event-1",
            "task-1",
            "session-1",
            1,
            2_000,
            EventSource::Runtime,
            EventVisibility::Ui,
            DomainEvent::TaskStarted,
        )];
        let logs = vec![RuntimeLogEntry {
            id: "log-1".into(),
            level: crate::runtime_log::RuntimeLogLevel::Info,
            message: "runtime booted".into(),
            task_id: Some("task-1".into()),
            timestamp_ms: 1_750,
        }];

        let queued = vec![QueuedTask {
            task_id: "task-1".into(),
            priority: TaskPriority::Normal,
            enqueued_at_ms: 1_500,
            background: false,
        }];
        let wakeups = vec![ScheduledWakeup {
            task_id: "task-1".into(),
            wake_at_ms: 5_000,
            reason: "retry".into(),
        }];
        let memory_records = vec![MemoryRecord {
            id: "memory-1".into(),
            kind: MemoryKind::Decision,
            scope: "workspace".into(),
            subject_ref: None,
            content: "Use TaskLoop".into(),
            source: "user".into(),
            confidence: 100,
            updated_at_ms: 3_000,
            supersedes: None,
            tags: vec!["naming".into()],
        }];

        store
            .save(
                Some(Path::new("/tmp/agentboard")),
                1,
                &sessions,
                &tasks,
                &artifacts,
                &subagents,
                &channel_objects,
                &work_items,
                &events,
                &logs,
                &queued,
                &wakeups,
                &memory_records,
            )
            .unwrap();
        let (
            workspace_root,
            next_sequence,
            loaded_sessions,
            loaded_tasks,
            loaded_artifacts,
            loaded_subagents,
            loaded_channel_objects,
            loaded_work_items,
            loaded_events,
            loaded_logs,
            loaded_queue,
            loaded_wakeups,
            loaded_memory,
        ) = store.load().unwrap();

        assert_eq!(workspace_root, Some(PathBuf::from("/tmp/agentboard")));
        assert_eq!(next_sequence, 1);
        assert_eq!(loaded_sessions, sessions);
        assert_eq!(loaded_tasks, tasks);
        assert_eq!(loaded_artifacts, artifacts);
        assert_eq!(loaded_subagents, subagents);
        assert_eq!(loaded_channel_objects, channel_objects);
        assert_eq!(loaded_work_items, work_items);
        assert_eq!(loaded_events, events);
        assert_eq!(loaded_logs, logs);
        assert_eq!(loaded_queue, queued);
        assert_eq!(loaded_wakeups, wakeups);
        assert_eq!(loaded_memory, memory_records);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn store_round_trips_connector_normalized_objects() {
        let root = temp_path("store-connector-roundtrip");
        let store = RuntimeStateStore::new(&root);
        let connector = ConnectorSample;
        let envelope = connector.normalize_external_task(
            "obj-1",
            "work-1",
            crate::connector::ChannelObjectPayload {
                source_system: "feishu".into(),
                object_type: "task".into(),
                external_id: "ext-1".into(),
                workspace_id: Some("space-1".into()),
                title: "Prepare report".into(),
                body: Some("Weekly summary".into()),
            },
        );

        let mut channel_objects = HashMap::new();
        channel_objects.insert(envelope.channel_object.id.clone(), envelope.channel_object);
        let mut work_items = HashMap::new();
        work_items.insert(envelope.work_item.id.clone(), envelope.work_item);

        store
            .save(
                Some(Path::new("/tmp/agentboard")),
                4,
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &channel_objects,
                &work_items,
                &[],
                &[],
                &[],
                &[],
                &[],
            )
            .unwrap();
        let (_, _, _, _, _, _, loaded_channel_objects, loaded_work_items, _, _, _, _, _) =
            store.load().unwrap();
        assert_eq!(loaded_channel_objects, channel_objects);
        assert_eq!(loaded_work_items, work_items);

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("agentboard-{prefix}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
