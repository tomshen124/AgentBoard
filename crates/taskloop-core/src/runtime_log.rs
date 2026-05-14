#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub id: String,
    pub level: RuntimeLogLevel,
    pub message: String,
    pub task_id: Option<String>,
    pub timestamp_ms: u64,
}

pub fn serialize_log_level(level: &RuntimeLogLevel) -> String {
    match level {
        RuntimeLogLevel::Debug => "debug".into(),
        RuntimeLogLevel::Info => "info".into(),
        RuntimeLogLevel::Warn => "warn".into(),
        RuntimeLogLevel::Error => "error".into(),
    }
}

pub fn parse_log_level(value: &str) -> Option<RuntimeLogLevel> {
    match value {
        "debug" => Some(RuntimeLogLevel::Debug),
        "info" => Some(RuntimeLogLevel::Info),
        "warn" => Some(RuntimeLogLevel::Warn),
        "error" => Some(RuntimeLogLevel::Error),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_log_level, serialize_log_level, RuntimeLogLevel};

    #[test]
    fn runtime_log_level_round_trip() {
        assert_eq!(parse_log_level(&serialize_log_level(&RuntimeLogLevel::Warn)), Some(RuntimeLogLevel::Warn));
    }
}
