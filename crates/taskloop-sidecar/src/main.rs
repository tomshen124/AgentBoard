//! TaskLoop Sidecar — JSON-RPC 2.0 server over stdin/stdout.
//!
//! Reads one JSON-RPC request per line from stdin,
//! writes one JSON-RPC response per line to stdout.
//! All diagnostic output goes to stderr.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use taskloop_core::bridge::{TaskCreationRequest, TaskLoopBridge};
use taskloop_core::execution::{ExecRequest, FileWriteRequest};
use taskloop_core::memory::MemoryKind;

// ── JSON-RPC wire types ──

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

// ── Method params ──

#[derive(Debug, Deserialize)]
struct InitParams { root: Option<String> }

#[derive(Debug, Deserialize)]
struct IdParams { id: String }

#[derive(Debug, Deserialize)]
struct CreateTaskParams { session_id: String, title: String, prompt: String }

#[derive(Debug, Deserialize)]
struct TaskIdParams { task_id: String }

#[derive(Debug, Deserialize)]
struct EvaluateExecParams { command: String }

#[derive(Debug, Deserialize)]
struct EvaluateFileWriteParams { path: String, #[serde(default)] destructive: bool }

#[derive(Debug, Deserialize)]
struct ReadFileParams {
    path: String,
    #[serde(default)] offset: usize,
    #[serde(default = "default_limit")] limit: usize,
}
fn default_limit() -> usize { 200 }

#[derive(Debug, Deserialize)]
struct ListDirParams { path: String }

#[derive(Debug, Deserialize)]
struct SearchRepoParams {
    query: String,
    path: String,
    #[serde(default = "default_max")] max_results: usize,
}
fn default_max() -> usize { 20 }

#[derive(Debug, Deserialize)]
struct RememberParams {
    kind: String,
    scope: String,
    content: String,
    #[serde(default)] tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RecallParams { scope: Option<String>, kind: Option<String> }

#[derive(Debug, Deserialize)]
struct AssembleContextParams { task_id: Option<String> }

#[derive(Debug, Deserialize)]
struct ExecParams {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)] approval_granted: bool,
}

#[derive(Debug, Deserialize)]
struct FileWriteParams {
    path: String,
    content: String,
    #[serde(default)] approval_granted: bool,
}

#[derive(Debug, Deserialize)]
struct ApproveRejectParams {
    task_id: String,
    checkpoint: String,
    #[serde(default)] reason: String,
}

fn main() {
    let mut bridge: Option<TaskLoopBridge> = None;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[sidecar] stdin error: {e}");
                return;
            }
        };
        if line.trim().is_empty() { continue; }

        let req: RpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                send_error(&mut out, Value::Null, -32700, &format!("Parse error: {e}"));
                continue;
            }
        };

        let result = dispatch(&req, &mut bridge);

        let resp = match result {
            Ok(val) => RpcResponse { jsonrpc: "2.0", id: req.id, result: Some(val), error: None },
            Err(msg) => RpcResponse { jsonrpc: "2.0", id: req.id, result: None, error: Some(RpcError { code: -32000, message: msg }) },
        };
        let _ = writeln!(out, "{}", serde_json::to_string(&resp).unwrap_or_default());
        let _ = out.flush();
    }
}

fn send_error(out: &mut impl Write, id: Value, code: i32, message: &str) {
    let resp = RpcResponse {
        jsonrpc: "2.0", id,
        result: None,
        error: Some(RpcError { code, message: message.to_string() }),
    };
    let _ = writeln!(out, "{}", serde_json::to_string(&resp).unwrap_or_default());
    let _ = out.flush();
}

fn dispatch(req: &RpcRequest, bridge: &mut Option<TaskLoopBridge>) -> Result<Value, String> {
    let params = req.params.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "workspace.init" => {
            let p: InitParams = from(params)?;
            let root = p.root.as_deref().unwrap_or(".");
            let b = TaskLoopBridge::new(PathBuf::from(root));
            let snap = b.snapshot();
            *bridge = Some(b);
            to_value(&snap)
        }
        "workspace.snapshot" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            to_value(&b.snapshot())
        }
        "session.ensure" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: IdParams = from(params)?;
            let s = b.ensure_session(&p.id);
            to_value(&s)
        }
        "task.create" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: CreateTaskParams = from(params)?;
            let t = b.create_task(TaskCreationRequest {
                session_id: p.session_id,
                title: p.title,
                prompt: p.prompt,
            }).map_err(|e| format!("{e:?}"))?;
            to_value(&t)
        }
        "task.projection" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: TaskIdParams = from(params)?;
            let proj = b.task_projection(&p.task_id).ok_or("task not found")?;
            to_value(proj)
        }
        "policy.evaluate_exec" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: EvaluateExecParams = from(params)?;
            to_value(&b.evaluate_exec(&p.command))
        }
        "policy.evaluate_file_write" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: EvaluateFileWriteParams = from(params)?;
            to_value(&b.evaluate_file_write(&PathBuf::from(&p.path), p.destructive))
        }
        "tools.read_file" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: ReadFileParams = from(params)?;
            let content = b.read_file(&PathBuf::from(&p.path), p.offset, p.limit)
                .map_err(|e| format!("{e:?}"))?;
            Ok(Value::String(content))
        }
        "tools.list_dir" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: ListDirParams = from(params)?;
            let entries = b.list_dir(&PathBuf::from(&p.path))
                .map_err(|e| format!("{e:?}"))?;
            to_value(&entries)
        }
        "tools.search_repo" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: SearchRepoParams = from(params)?;
            let results = b.search_repo(&p.query, &PathBuf::from(&p.path), p.max_results)
                .map_err(|e| format!("{e:?}"))?;
            to_value(&results)
        }
        "tools.exec" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: ExecParams = from(params)?;
            let req = ExecRequest {
                program: p.program,
                args: p.args,
                cwd: p.cwd.map(PathBuf::from),
                approval_granted: p.approval_granted,
            };
            to_value(&b.execute_command(req).map_err(|e| format!("{e:?}"))?)
        }
        "tools.write_file" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: FileWriteParams = from(params)?;
            let req = FileWriteRequest {
                path: PathBuf::from(&p.path),
                content: p.content.into_bytes(),
                approval_granted: p.approval_granted,
            };
            to_value(&b.execute_file_write(req).map_err(|e| format!("{e:?}"))?)
        }
        "memory.remember" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: RememberParams = from(params)?;
            let kind = parse_memory_kind(&p.kind).ok_or_else(|| format!("unknown kind: {}", p.kind))?;
            b.remember(kind, &p.scope, &p.content, p.tags);
            Ok(Value::String("ok".into()))
        }
        "memory.recall" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: RecallParams = from(params)?;
            let kind = p.kind.as_deref().and_then(parse_memory_kind);
            let records: Vec<_> = b.recall(p.scope.as_deref(), kind).into_iter().cloned().collect();
            to_value(&records)
        }
        "context.assemble" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            let p: AssembleContextParams = from(params)?;
            to_value(&b.assemble_context(p.task_id.as_deref()).map_err(|e| format!("{e:?}"))?)
        }
        "approval.approve" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: ApproveRejectParams = from(params)?;
            b.approve_exec(&p.task_id, &p.checkpoint).map_err(|e| format!("{e:?}"))?;
            Ok(Value::String("ok".into()))
        }
        "approval.reject" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let p: ApproveRejectParams = from(params)?;
            b.reject_exec(&p.task_id, &p.checkpoint, &p.reason).map_err(|e| format!("{e:?}"))?;
            Ok(Value::String("ok".into()))
        }
        "state.save" => {
            let b = bridge.as_ref().ok_or("not initialized")?;
            b.save().map_err(|e| format!("{e:?}"))?;
            Ok(Value::String("ok".into()))
        }
        "state.load" => {
            let b = bridge.as_mut().ok_or("not initialized")?;
            let loaded = b.load().map_err(|e| format!("{e:?}"))?;
            Ok(Value::Bool(loaded))
        }
        other => Err(format!("unknown method: {other}")),
    }
}

fn from<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("bad params: {e}"))
}

fn to_value(v: &impl Serialize) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| format!("serialize: {e}"))
}

fn parse_memory_kind(raw: &str) -> Option<MemoryKind> {
    match raw {
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
