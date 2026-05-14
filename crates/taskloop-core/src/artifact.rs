use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    execution::RuntimeActionError,
    model::Artifact,
};

pub const DEFAULT_ARTIFACT_TTL_MS: u64 = 24 * 60 * 60 * 1000;

pub fn default_artifact_cache_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".taskloop").join("tmp")
}

pub fn default_artifact_export_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".taskloop").join("exports")
}

pub fn stage_artifact(
    workspace_root: &Path,
    artifact_id: impl Into<String>,
    task_id: impl Into<String>,
    name: impl Into<String>,
    content: &[u8],
    now_ms: u64,
    ttl_ms: Option<u64>,
) -> Result<Artifact, RuntimeActionError> {
    let artifact_id = artifact_id.into();
    let task_id = task_id.into();
    let name = name.into();
    let cache_root = default_artifact_cache_root(workspace_root);
    fs::create_dir_all(&cache_root)?;

    let safe_name = sanitize_filename(&name);
    let path = cache_root.join(format!("{artifact_id}-{safe_name}"));
    fs::write(&path, content)?;

    Ok(Artifact {
        id: artifact_id,
        task_id,
        name,
        path: path.to_string_lossy().to_string(),
        expires_at_ms: Some(now_ms.saturating_add(ttl_ms.unwrap_or(DEFAULT_ARTIFACT_TTL_MS))),
    })
}

pub fn cleanup_expired_artifacts(
    artifacts: &mut HashMap<String, Artifact>,
    now_ms: u64,
) -> Result<Vec<String>, RuntimeActionError> {
    let expired_ids = artifacts
        .values()
        .filter(|artifact| artifact.expires_at_ms.is_some_and(|expires| expires <= now_ms))
        .map(|artifact| artifact.id.clone())
        .collect::<Vec<_>>();

    for artifact_id in &expired_ids {
        if let Some(artifact) = artifacts.remove(artifact_id) {
            let path = PathBuf::from(&artifact.path);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
    }

    Ok(expired_ids)
}

pub fn export_artifact(
    artifact: &Artifact,
    destination_path: &Path,
) -> Result<PathBuf, RuntimeActionError> {
    let source = PathBuf::from(&artifact.path);
    if !source.exists() {
        return Err(RuntimeActionError::Io(format!(
            "artifact source does not exist: {}",
            source.display()
        )));
    }

    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, destination_path)?;
    Ok(destination_path.to_path_buf())
}

fn sanitize_filename(value: &str) -> String {
    value.chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '\0' => '_',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        cleanup_expired_artifacts, default_artifact_export_root, export_artifact, stage_artifact,
        DEFAULT_ARTIFACT_TTL_MS,
    };

    #[test]
    fn staged_artifact_gets_default_ttl_and_writes_file() {
        let root = temp_dir("artifact-stage");
        let artifact = stage_artifact(
            &root,
            "artifact-1",
            "task-1",
            "report.md",
            b"hello",
            1_000,
            None,
        )
        .unwrap();

        assert!(artifact.path.contains(".taskloop/tmp"));
        assert_eq!(artifact.expires_at_ms, Some(1_000 + DEFAULT_ARTIFACT_TTL_MS));
        assert_eq!(fs::read_to_string(&artifact.path).unwrap(), "hello");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup_removes_expired_artifact_files() {
        let root = temp_dir("artifact-cleanup");
        let artifact = stage_artifact(
            &root,
            "artifact-1",
            "task-1",
            "report.md",
            b"hello",
            1_000,
            Some(10),
        )
        .unwrap();
        let artifact_path = artifact.path.clone();
        let mut artifacts = HashMap::new();
        artifacts.insert(artifact.id.clone(), artifact);

        let removed = cleanup_expired_artifacts(&mut artifacts, 1_020).unwrap();
        assert_eq!(removed, vec!["artifact-1"]);
        assert!(!PathBuf::from(artifact_path).exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn artifact_can_be_exported_to_explicit_destination() {
        let root = temp_dir("artifact-export");
        let artifact = stage_artifact(
            &root,
            "artifact-1",
            "task-1",
            "report.md",
            b"hello",
            1_000,
            None,
        )
        .unwrap();

        let export_path = default_artifact_export_root(&root).join("report-copy.md");
        let exported = export_artifact(&artifact, &export_path).unwrap();

        assert_eq!(exported, export_path);
        assert_eq!(fs::read_to_string(exported).unwrap(), "hello");

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agentboard-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
