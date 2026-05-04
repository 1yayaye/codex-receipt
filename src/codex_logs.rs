//! 负责读取 Codex 本地会话日志，并把 JSONL 事件转换成稳定的用量快照。
//! 本模块不负责价格估算，也不负责小票排版。

use crate::models::{Scope, UsageSnapshot};
use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct SessionCandidate {
    path: PathBuf,
    session_id: Option<String>,
    token_timestamp_ms: Option<i64>,
    modified: Option<SystemTime>,
}

/// 查找最近的可用 Codex 会话 JSONL 文件。
///
/// 会递归扫描 Codex 的 `sessions` 与 `archived_sessions` 目录。默认跳过当前正在运行
/// `codex-receipt` 的 Codex 线程，避免工具自身产生的日志覆盖刚结束的目标会话；如果没有
/// 其它可用会话，则回退到当前线程。找不到含 `token_count` 的文件时返回错误。
pub fn newest_session_file() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("无法定位用户主目录"))?;

    let mut files = Vec::new();
    for root in [
        home.join(".codex").join("sessions"),
        home.join(".codex").join("archived_sessions"),
    ] {
        collect_jsonl_files(&root, &mut files)?;
    }

    let mut candidates = Vec::new();
    for path in files {
        if let Ok(Some(candidate)) = session_candidate(&path) {
            candidates.push(candidate);
        }
    }

    let current_thread_id = std::env::var("CODEX_THREAD_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let selectable = match current_thread_id.as_deref() {
        Some(thread_id) => {
            let non_current = candidates
                .iter()
                .filter(|candidate| !is_current_thread(candidate, thread_id))
                .cloned()
                .collect::<Vec<_>>();
            if non_current.is_empty() {
                candidates
            } else {
                non_current
            }
        }
        None => candidates,
    };

    selectable
        .into_iter()
        .max_by_key(|candidate| (candidate.token_timestamp_ms, candidate.modified))
        .map(|candidate| candidate.path)
        .ok_or_else(|| {
            anyhow!("未在 ~/.codex/sessions 或 ~/.codex/archived_sessions 下找到含 token_count 的 Codex 会话文件")
        })
}

/// 从指定 Codex JSONL 文件读取用量快照。
///
/// `scope` 决定读取最近一轮还是整场会话。缺少 `token_count` 事件或目标用量字段时返回错误，
/// 调用方应把错误展示成可操作的 CLI 提示。
pub fn load_snapshot_from_session(
    path: &Path,
    scope: Scope,
    model_override: Option<&str>,
    provider_override: Option<&str>,
) -> Result<UsageSnapshot> {
    let file = File::open(path).with_context(|| format!("无法打开会话文件 {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("codex-session")
        .to_string();
    let mut provider = "unknown".to_string();
    let mut model: Option<String> = None;
    let mut timestamp: Option<String> = None;
    let mut token_info: Option<Value> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let item: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
        let null_payload = Value::Null;
        let payload = item.get("payload").unwrap_or(&null_payload);

        if item_type == "session_meta" {
            if let Some(id) = payload.get("id").and_then(Value::as_str) {
                session_id = id.to_string();
            }
            if let Some(value) = payload.get("model_provider").and_then(Value::as_str) {
                provider = value.to_string();
            }
            for key in ["model", "model_id", "model_name", "model_slug"] {
                if model.is_none() {
                    model = payload.get(key).and_then(Value::as_str).map(str::to_string);
                }
            }
        }

        if item_type == "turn_context" && model.is_none() {
            model = payload
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string);
        }

        if item_type == "event_msg"
            && payload.get("type").and_then(Value::as_str) == Some("token_count")
        {
            token_info = payload.get("info").cloned();
            timestamp = item
                .get("timestamp")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }

    let info = token_info.ok_or_else(|| anyhow!("会话文件中没有 token_count 事件"))?;
    let usage_key = match scope {
        Scope::LatestTurn => "last_token_usage",
        Scope::Session => "total_token_usage",
    };
    let usage = info
        .get(usage_key)
        .ok_or_else(|| anyhow!("token_count 事件中没有 {}", usage_key))?;

    let available_fields = usage
        .as_object()
        .map(|obj| {
            let mut keys = obj.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        })
        .unwrap_or_default();

    let input_tokens = u64_field(usage, "input_tokens");
    let output_tokens = u64_field(usage, "output_tokens");
    let total_tokens = u64_field_opt(usage, "total_tokens").unwrap_or(input_tokens + output_tokens);
    let context_input_tokens = info
        .get("last_token_usage")
        .and_then(|last_usage| u64_field_opt(last_usage, "input_tokens"))
        .unwrap_or(input_tokens);

    Ok(UsageSnapshot {
        input_tokens,
        context_input_tokens,
        cached_input_tokens: u64_field(usage, "cached_input_tokens"),
        output_tokens,
        reasoning_output_tokens: u64_field(usage, "reasoning_output_tokens"),
        total_tokens,
        context_window: u64_field_opt(&info, "model_context_window"),
        provider: provider_override.unwrap_or(&provider).to_string(),
        model: model_override
            .map(str::to_string)
            .or(model)
            .unwrap_or_else(|| "模型未记录".to_string()),
        source: path.to_path_buf(),
        session_id,
        timestamp,
        scope: match scope {
            Scope::LatestTurn => "latest-turn".to_string(),
            Scope::Session => "session".to_string(),
        },
        available_fields,
    })
}

fn collect_jsonl_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(root).with_context(|| format!("无法读取 {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }

    Ok(())
}

fn session_candidate(path: &Path) -> Result<Option<SessionCandidate>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut session_id = None;
    let mut token_timestamp_ms = None;
    let mut has_token_count = false;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let item: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
        let null_payload = Value::Null;
        let payload = item.get("payload").unwrap_or(&null_payload);

        if item_type == "session_meta" && session_id.is_none() {
            session_id = payload
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string);
        }

        if item_type == "event_msg"
            && payload.get("type").and_then(Value::as_str) == Some("token_count")
        {
            has_token_count = true;
            token_timestamp_ms = item
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_timestamp_ms);
        }
    }

    if !has_token_count {
        return Ok(None);
    }

    Ok(Some(SessionCandidate {
        path: path.to_path_buf(),
        session_id,
        token_timestamp_ms,
        modified: path.metadata().and_then(|meta| meta.modified()).ok(),
    }))
}

fn is_current_thread(candidate: &SessionCandidate, thread_id: &str) -> bool {
    candidate.session_id.as_deref() == Some(thread_id)
        || candidate
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(thread_id))
}

fn parse_timestamp_ms(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn u64_field(value: &Value, key: &str) -> u64 {
    u64_field_opt(value, key).unwrap_or(0)
}

fn u64_field_opt(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}
