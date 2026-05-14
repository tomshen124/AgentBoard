use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    TaskState,
    Decision,
    ProjectContext,
    Fact,
    Preference,
    ExternalReference,
    ArtifactSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub kind: MemoryKind,
    pub scope: String,
    pub subject_ref: Option<String>,
    pub content: String,
    pub source: String,
    pub confidence: u8,
    pub updated_at_ms: u64,
    pub supersedes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryStore {
    records: BTreeMap<String, MemoryRecord>,
}

impl MemoryStore {
    pub fn insert(&mut self, record: MemoryRecord) {
        self.records.insert(record.id.clone(), record);
    }

    pub fn get(&self, record_id: &str) -> Option<&MemoryRecord> {
        self.records.get(record_id)
    }

    pub fn all(&self) -> Vec<&MemoryRecord> {
        self.records.values().collect()
    }

    pub fn active(&self) -> Vec<&MemoryRecord> {
        let superseded = self
            .records
            .values()
            .filter_map(|record| record.supersedes.clone())
            .collect::<HashSet<_>>();

        self.records
            .values()
            .filter(|record| !superseded.contains(&record.id))
            .collect()
    }

    pub fn for_scope(&self, scope: &str) -> Vec<&MemoryRecord> {
        self.records
            .values()
            .filter(|record| record.scope == scope)
            .collect()
    }

    pub fn from_records(records: Vec<MemoryRecord>) -> Self {
        let mut store = Self::default();
        for record in records {
            store.insert(record);
        }
        store
    }
}

pub fn render_memory_markdown(
    records: &[&MemoryRecord],
    updated_at: &str,
    scope: &str,
    title: &str,
) -> String {
    let mut sorted = records.iter().map(|record| *record).collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        serialize_memory_kind(&left.kind)
            .cmp(&serialize_memory_kind(&right.kind))
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("kind: memory\n");
    out.push_str("version: 1\n");
    out.push_str(&format!("scope: {scope}\n"));
    out.push_str(&format!("updated_at: {updated_at}\n"));
    out.push_str("source_of_truth: structured_projection\n");
    out.push_str("visibility: internal\n");
    out.push_str("---\n\n");
    out.push_str(&format!("# {title}\n\n"));

    if sorted.is_empty() {
        out.push_str("- No active structured memory records.\n");
        return out;
    }

    let mut current_kind = String::new();
    for record in sorted {
        let kind = serialize_memory_kind(&record.kind);
        if kind != current_kind {
            if !current_kind.is_empty() {
                out.push('\n');
            }
            current_kind = kind.clone();
            out.push_str(&format!("## {}\n", humanize_memory_kind(&kind)));
        }
        let subject = record
            .subject_ref
            .as_ref()
            .map(|value| format!(" ({value})"))
            .unwrap_or_default();
        let tags = if record.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", record.tags.join(", "))
        };
        out.push_str(&format!(
            "- {}{}{} source={} confidence={}\n",
            record.content, subject, tags, record.source, record.confidence
        ));
    }

    out
}

pub fn serialize_memory_kind(kind: &MemoryKind) -> String {
    match kind {
        MemoryKind::TaskState => "task_state".into(),
        MemoryKind::Decision => "decision".into(),
        MemoryKind::ProjectContext => "project_context".into(),
        MemoryKind::Fact => "fact".into(),
        MemoryKind::Preference => "preference".into(),
        MemoryKind::ExternalReference => "external_reference".into(),
        MemoryKind::ArtifactSummary => "artifact_summary".into(),
    }
}

pub fn parse_memory_kind(value: &str) -> Option<MemoryKind> {
    match value {
        "task_state" => Some(MemoryKind::TaskState),
        "decision" => Some(MemoryKind::Decision),
        "project_context" => Some(MemoryKind::ProjectContext),
        "fact" => Some(MemoryKind::Fact),
        "preference" => Some(MemoryKind::Preference),
        "external_reference" => Some(MemoryKind::ExternalReference),
        "artifact_summary" => Some(MemoryKind::ArtifactSummary),
        _ => None,
    }
}

fn humanize_memory_kind(value: &str) -> String {
    value
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.push(first.to_ascii_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{render_memory_markdown, MemoryKind, MemoryRecord, MemoryStore};

    #[test]
    fn active_memory_filters_superseded_records() {
        let mut store = MemoryStore::default();
        store.insert(MemoryRecord {
            id: "memory-1".into(),
            kind: MemoryKind::Fact,
            scope: "workspace".into(),
            subject_ref: None,
            content: "Old fact".into(),
            source: "user".into(),
            confidence: 80,
            updated_at_ms: 1_000,
            supersedes: None,
            tags: vec!["project".into()],
        });
        store.insert(MemoryRecord {
            id: "memory-2".into(),
            kind: MemoryKind::Fact,
            scope: "workspace".into(),
            subject_ref: None,
            content: "New fact".into(),
            source: "user".into(),
            confidence: 95,
            updated_at_ms: 2_000,
            supersedes: Some("memory-1".into()),
            tags: vec!["project".into()],
        });

        let active = store.active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "memory-2");
    }

    #[test]
    fn memory_projection_renders_frontmatter_and_sections() {
        let mut store = MemoryStore::default();
        store.insert(MemoryRecord {
            id: "memory-1".into(),
            kind: MemoryKind::Fact,
            scope: "workspace".into(),
            subject_ref: None,
            content: "TaskLoop owns structured state".into(),
            source: "user".into(),
            confidence: 95,
            updated_at_ms: 2_000,
            supersedes: None,
            tags: vec!["architecture".into()],
        });
        store.insert(MemoryRecord {
            id: "memory-2".into(),
            kind: MemoryKind::Decision,
            scope: "workspace".into(),
            subject_ref: Some("naming".into()),
            content: "AgentBoard is the product name".into(),
            source: "user".into(),
            confidence: 100,
            updated_at_ms: 3_000,
            supersedes: None,
            tags: vec![],
        });

        let active = store.active();
        let rendered =
            render_memory_markdown(&active, "2026-03-25", "workspace", "Stable Workspace Facts");

        assert!(rendered.contains("source_of_truth: structured_projection"));
        assert!(rendered.contains("# Stable Workspace Facts"));
        assert!(rendered.contains("## Decision"));
        assert!(rendered.contains("## Fact"));
        assert!(rendered.contains("AgentBoard is the product name (naming)"));
    }
}
