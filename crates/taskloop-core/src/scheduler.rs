use crate::model::TaskPriority;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedTask {
    pub task_id: String,
    pub priority: TaskPriority,
    pub enqueued_at_ms: u64,
    pub background: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledWakeup {
    pub task_id: String,
    pub wake_at_ms: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskQueue {
    queued: Vec<QueuedTask>,
    wakeups: Vec<ScheduledWakeup>,
}

impl TaskQueue {
    pub fn enqueue(&mut self, entry: QueuedTask) {
        self.queued.push(entry);
        self.queued.sort_by(queue_order);
    }

    pub fn dequeue_next(&mut self) -> Option<QueuedTask> {
        if self.queued.is_empty() {
            None
        } else {
            Some(self.queued.remove(0))
        }
    }

    pub fn queued(&self) -> &[QueuedTask] {
        &self.queued
    }

    pub fn schedule_wakeup(&mut self, wakeup: ScheduledWakeup) {
        self.wakeups.push(wakeup);
        self.wakeups.sort_by(|left, right| {
            left.wake_at_ms
                .cmp(&right.wake_at_ms)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
    }

    pub fn drain_due_wakeups(&mut self, now_ms: u64) -> Vec<ScheduledWakeup> {
        let split = self
            .wakeups
            .iter()
            .take_while(|item| item.wake_at_ms <= now_ms)
            .count();
        self.wakeups.drain(0..split).collect()
    }

    pub fn wakeups(&self) -> &[ScheduledWakeup] {
        &self.wakeups
    }

    pub fn remove_task(&mut self, task_id: &str) -> bool {
        let before = self.queued.len();
        self.queued.retain(|entry| entry.task_id != task_id);
        before != self.queued.len()
    }

    pub fn remove_wakeups_for_task(&mut self, task_id: &str) -> usize {
        let before = self.wakeups.len();
        self.wakeups.retain(|entry| entry.task_id != task_id);
        before.saturating_sub(self.wakeups.len())
    }

    pub fn from_parts(queued: Vec<QueuedTask>, wakeups: Vec<ScheduledWakeup>) -> Self {
        let mut queue = Self { queued, wakeups };
        queue.queued.sort_by(queue_order);
        queue.wakeups.sort_by(|left, right| {
            left.wake_at_ms
                .cmp(&right.wake_at_ms)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        queue
    }
}

fn queue_order(left: &QueuedTask, right: &QueuedTask) -> std::cmp::Ordering {
    priority_rank(&right.priority)
        .cmp(&priority_rank(&left.priority))
        .then_with(|| left.enqueued_at_ms.cmp(&right.enqueued_at_ms))
        .then_with(|| left.task_id.cmp(&right.task_id))
}

fn priority_rank(priority: &TaskPriority) -> u8 {
    match priority {
        TaskPriority::Low => 0,
        TaskPriority::Normal => 1,
        TaskPriority::High => 2,
    }
}

#[cfg(test)]
mod tests {
    use crate::model::TaskPriority;

    use super::{QueuedTask, ScheduledWakeup, TaskQueue};

    #[test]
    fn queue_prefers_high_priority_then_older_items() {
        let mut queue = TaskQueue::default();
        queue.enqueue(QueuedTask {
            task_id: "task-low".into(),
            priority: TaskPriority::Low,
            enqueued_at_ms: 2_000,
            background: false,
        });
        queue.enqueue(QueuedTask {
            task_id: "task-high".into(),
            priority: TaskPriority::High,
            enqueued_at_ms: 3_000,
            background: false,
        });
        queue.enqueue(QueuedTask {
            task_id: "task-normal".into(),
            priority: TaskPriority::Normal,
            enqueued_at_ms: 1_000,
            background: false,
        });

        assert_eq!(queue.dequeue_next().unwrap().task_id, "task-high");
        assert_eq!(queue.dequeue_next().unwrap().task_id, "task-normal");
        assert_eq!(queue.dequeue_next().unwrap().task_id, "task-low");
    }

    #[test]
    fn wakeup_queue_returns_due_items_in_order() {
        let mut queue = TaskQueue::default();
        queue.schedule_wakeup(ScheduledWakeup {
            task_id: "task-2".into(),
            wake_at_ms: 3_000,
            reason: "later".into(),
        });
        queue.schedule_wakeup(ScheduledWakeup {
            task_id: "task-1".into(),
            wake_at_ms: 2_000,
            reason: "earlier".into(),
        });

        let due = queue.drain_due_wakeups(2_500);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].task_id, "task-1");
        assert_eq!(queue.wakeups().len(), 1);
    }
}
