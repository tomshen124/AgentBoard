use crate::{
    event::EventEnvelope,
    runtime_log::RuntimeLogEntry,
    state::{TaskHealthState, TaskLifecycleState, TaskProjection, TaskWorkflowMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDiagnosticReport {
    pub task_id: String,
    pub lifecycle: TaskLifecycleState,
    pub workflow_mode: TaskWorkflowMode,
    pub health: TaskHealthState,
    pub summary: String,
    pub blocked_reason: Option<String>,
    pub recent_events: Vec<String>,
    pub recent_logs: Vec<String>,
    pub recommendations: Vec<String>,
}

pub fn build_task_diagnostic_report(
    projection: &TaskProjection,
    events: &[EventEnvelope],
    logs: &[RuntimeLogEntry],
) -> TaskDiagnosticReport {
    let recent_events = events
        .iter()
        .rev()
        .filter(|event| event.task_id.as_deref() == Some(projection.task_id.as_str()))
        .take(5)
        .map(|event| format!("{:?}", event.event))
        .collect::<Vec<_>>();

    let recent_logs = logs
        .iter()
        .rev()
        .filter(|log| log.task_id.as_deref() == Some(projection.task_id.as_str()) || log.task_id.is_none())
        .take(5)
        .map(|log| format!("{:?}: {}", log.level, log.message))
        .collect::<Vec<_>>();

    let mut recommendations = Vec::new();
    if matches!(projection.health, TaskHealthState::Stalled) {
        recommendations.push("check last tool action or issue a retry/resume".into());
    }
    if matches!(projection.lifecycle, TaskLifecycleState::Waiting) {
        recommendations.push("inspect waiting reason and unblock approval/input/schedule dependency".into());
    }
    if matches!(projection.workflow_mode, TaskWorkflowMode::Complex) {
        recommendations.push("inspect planner/executor/reviewer progression before forcing retry".into());
    }
    if projection.blocked_reason.is_some() && recommendations.is_empty() {
        recommendations.push("inspect blocked reason and recent policy or tool events".into());
    }

    let summary = format!(
        "task={} lifecycle={:?} workflow={:?} health={:?}",
        projection.task_id, projection.lifecycle, projection.workflow_mode, projection.health
    );

    TaskDiagnosticReport {
        task_id: projection.task_id.clone(),
        lifecycle: projection.lifecycle.clone(),
        workflow_mode: projection.workflow_mode.clone(),
        health: projection.health.clone(),
        summary,
        blocked_reason: projection.blocked_reason.clone(),
        recent_events,
        recent_logs,
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        diagnostics::build_task_diagnostic_report,
        event::{DomainEvent, EventEnvelope, EventSource, EventVisibility},
        model::{Task, TaskHost, TaskKind, TaskPriority},
        runtime_log::{RuntimeLogEntry, RuntimeLogLevel},
        state::{TaskHealthState, TaskProjection, TaskWorkflowMode},
    };

    #[test]
    fn diagnostic_report_includes_recommendations_for_stalled_complex_task() {
        let task = Task {
            id: "task-1".into(),
            session_id: "session-1".into(),
            work_item_id: None,
            title: "Example".into(),
            kind: TaskKind::Internal,
            host: TaskHost::Internal,
            priority: TaskPriority::Normal,
            created_at_ms: 1_000,
        };
        let mut projection = TaskProjection::from_task(&task);
        projection.workflow_mode = TaskWorkflowMode::Complex;
        projection.health = TaskHealthState::Stalled;
        projection.blocked_reason = Some("review pending".into());
        let events = vec![EventEnvelope::for_task(
            "event-1",
            "task-1",
            "session-1",
            1,
            2_000,
            EventSource::Runtime,
            EventVisibility::Ui,
            DomainEvent::TaskBlocked {
                reason: "review pending".into(),
            },
        )];
        let logs = vec![RuntimeLogEntry {
            id: "log-1".into(),
            level: RuntimeLogLevel::Warn,
            message: "review has not advanced".into(),
            task_id: Some("task-1".into()),
            timestamp_ms: 2_100,
        }];

        let report = build_task_diagnostic_report(&projection, &events, &logs);
        assert!(report.summary.contains("workflow=Complex"));
        assert!(!report.recommendations.is_empty());
        assert_eq!(report.blocked_reason.as_deref(), Some("review pending"));
    }
}
