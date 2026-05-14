use serde::{Deserialize, Serialize};
use crate::{
    contracts::MarkdownContract,
    execution::RuntimeActionError,
    memory::{serialize_memory_kind, MemoryRecord},
    runtime::TaskLoopRuntime,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptContextSection {
    pub key: String,
    pub title: String,
    pub source: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptContextBundle {
    pub generated_at_ms: u64,
    pub task_id: Option<String>,
    pub sections: Vec<PromptContextSection>,
}

impl PromptContextBundle {
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Prompt Context Bundle\n\n");
        out.push_str(&format!("generated_at_ms: {}\n\n", self.generated_at_ms));

        if let Some(task_id) = &self.task_id {
            out.push_str(&format!("task_id: {task_id}\n\n"));
        }

        for section in &self.sections {
            out.push_str(&format!("## {}\n", section.title));
            out.push_str(&format!("source: {}\n\n", section.source));
            out.push_str(section.content.trim());
            out.push_str("\n\n");
        }

        out.trim_end().to_string()
    }
}

pub fn assemble_prompt_context(
    runtime: &TaskLoopRuntime,
    task_id: Option<&str>,
    generated_at_ms: u64,
) -> Result<PromptContextBundle, RuntimeActionError> {
    let mut sections = Vec::new();

    if let Some(contracts) = runtime.workspace_contracts() {
        push_contract_section(&mut sections, "agents", "Operating Rules", contracts.agents.as_ref());
        push_contract_section(&mut sections, "tools", "Tooling Notes", contracts.tools.as_ref());
        push_contract_section(&mut sections, "profile", "User Profile", contracts.profile.as_ref());
        push_contract_section(&mut sections, "focus", "Current Focus", contracts.focus.as_ref());
    }

    let workspace_memory = runtime
        .memory_store()
        .active()
        .into_iter()
        .filter(|record| matches!(record.scope.as_str(), "workspace" | "project"))
        .collect::<Vec<_>>();
    if !workspace_memory.is_empty() {
        sections.push(PromptContextSection {
            key: "workspace_memory".into(),
            title: "Stable Workspace Memory".into(),
            source: "structured_memory".into(),
            content: render_memory_context(&workspace_memory),
        });
    }

    if let Some(task_id) = task_id {
        let projection = runtime
            .task_state(task_id)
            .ok_or_else(|| RuntimeActionError::UnknownTask(task_id.to_string()))?;
        sections.push(PromptContextSection {
            key: "task_state".into(),
            title: "Task State".into(),
            source: "task_projection".into(),
            content: format!(
                "title: {}\nlifecycle: {:?}\nworkflow_mode: {:?}\nhealth: {:?}\nprogress: {}\nblocked_reason: {}\nwaiting_on: {}\nresume_checkpoint: {}\nactive_subagents: {}",
                projection.title,
                projection.lifecycle,
                projection.workflow_mode,
                projection.health,
                projection
                    .progress_text
                    .clone()
                    .unwrap_or_else(|| "none".into()),
                projection
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| "none".into()),
                projection
                    .waiting_on
                    .as_ref()
                    .map(|value| format!("{value:?}"))
                    .unwrap_or_else(|| "none".into()),
                projection
                    .resume_checkpoint
                    .clone()
                    .unwrap_or_else(|| "none".into()),
                projection.active_subagents
            ),
        });

        let task_memory = runtime
            .memory_store()
            .active()
            .into_iter()
            .filter(|record| {
                record.scope == task_id || record.subject_ref.as_deref() == Some(task_id)
            })
            .collect::<Vec<_>>();
        if !task_memory.is_empty() {
            sections.push(PromptContextSection {
                key: "task_memory".into(),
                title: "Task Memory".into(),
                source: "structured_memory".into(),
                content: render_memory_context(&task_memory),
            });
        }
    }

    Ok(PromptContextBundle {
        generated_at_ms,
        task_id: task_id.map(|value| value.to_string()),
        sections,
    })
}

fn push_contract_section(
    sections: &mut Vec<PromptContextSection>,
    key: &str,
    title: &str,
    contract: Option<&MarkdownContract>,
) {
    let Some(contract) = contract else {
        return;
    };
    let content = contract.body.trim();
    if content.is_empty() {
        return;
    }

    sections.push(PromptContextSection {
        key: key.to_string(),
        title: title.to_string(),
        source: contract.path.to_string_lossy().to_string(),
        content: content.to_string(),
    });
}

fn render_memory_context(records: &[&MemoryRecord]) -> String {
    let mut sorted = records.to_vec();
    sorted.sort_by(|left, right| {
        left.scope
            .cmp(&right.scope)
            .then_with(|| serialize_memory_kind(&left.kind).cmp(&serialize_memory_kind(&right.kind)))
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
            .then_with(|| left.id.cmp(&right.id))
    });

    sorted
        .into_iter()
        .map(|record| {
            let subject = record
                .subject_ref
                .as_ref()
                .map(|value| format!(" subject={value}"))
                .unwrap_or_default();
            format!(
                "- [{}] scope={} confidence={}{} {}",
                serialize_memory_kind(&record.kind),
                record.scope,
                record.confidence,
                subject,
                record.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        context::assemble_prompt_context,
        memory::{MemoryKind, MemoryRecord},
        model::{Session, Task, TaskHost, TaskKind, TaskPriority},
        runtime::TaskLoopRuntime,
        WorkspaceLayout,
    };

    #[test]
    fn assembled_prompt_context_includes_contracts_and_memory() {
        let temp_dir = create_temp_workspace("prompt-context");
        fs::write(
            temp_dir.join("AGENTS.md"),
            r#"---
kind: agents
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: markdown_contract
visibility: internal
---

Prefer explicit progress reporting.
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("TOOLS.md"),
            r#"---
kind: tools
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: markdown_contract
visibility: internal
---

Use rg before slower file scans.
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("PROFILE.md"),
            r#"---
kind: profile
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: user_editable_contract
visibility: internal
---

Keep answers concise and operational.
"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("FOCUS.md"),
            r#"---
kind: focus
version: 1
scope: workspace
updated_at: 2026-04-03
source_of_truth: user_editable_contract
visibility: internal
---

Ship the desktop shell before broadening scope.
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
            title: "Build desktop shell".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::High,
            created_at_ms: 1_000,
        });
        runtime.remember(MemoryRecord {
            id: "memory-1".into(),
            kind: MemoryKind::Preference,
            scope: "workspace".into(),
            subject_ref: None,
            content: "Favor local-first desktop delivery".into(),
            source: "user".into(),
            confidence: 95,
            updated_at_ms: 1_200,
            supersedes: None,
            tags: vec!["product".into()],
        });

        let bundle = assemble_prompt_context(&runtime, Some("task-1"), 2_000).unwrap();

        assert!(bundle.sections.iter().any(|item| item.key == "agents"));
        assert!(bundle.sections.iter().any(|item| item.key == "profile"));
        assert!(bundle.sections.iter().any(|item| item.key == "focus"));
        assert!(bundle.sections.iter().any(|item| item.key == "workspace_memory"));
        assert!(bundle.sections.iter().any(|item| item.key == "task_state"));
        assert!(bundle
            .render_markdown()
            .contains("Ship the desktop shell before broadening scope."));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn assembled_prompt_context_errors_for_unknown_task() {
        let runtime = TaskLoopRuntime::new();
        let error = assemble_prompt_context(&runtime, Some("missing-task"), 2_000).unwrap_err();
        assert!(matches!(error, crate::execution::RuntimeActionError::UnknownTask(_)));
    }

    fn create_temp_workspace(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agentboard-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
