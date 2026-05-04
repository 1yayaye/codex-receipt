//! 负责后台监听 Codex 归档会话，并在会话完成后生成 HTML 小票。
//! 本模块不解析 JSONL 细节、不估算价格细则，也不直接调用系统通知 API。

use crate::desktop_notify::{NotificationMessage, Notifier};
use crate::models::{Scope, UsageSnapshot};
use crate::opener::Opener;
use crate::watch_state::{ProcessedSession, WatchState};
use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub archived_dir: PathBuf,
    pub receipt_dir: PathBuf,
    pub state_path: PathBuf,
    pub pricing_path: PathBuf,
    pub replay_existing: bool,
    pub once: bool,
    pub open_receipt: bool,
}

#[derive(Debug, Clone)]
pub struct NotifyTestReport {
    pub receipt_path: PathBuf,
    pub notification_status: String,
    pub open_status: String,
}

/// 执行 watch 模式的启动扫描。
///
/// 默认只把现有有效归档写入状态，不生成小票也不通知；`replay_existing` 为真时会处理尚未记录的历史归档。
pub fn run_initial_scan(
    config: &WatchConfig,
    notifier: &dyn Notifier,
    opener: &dyn Opener,
) -> Result<usize> {
    let sessions = archived_session_files(&config.archived_dir)?;
    if config.replay_existing {
        let mut processed = 0;
        for session in sessions {
            match process_archived_session(&session, config, notifier, opener) {
                Ok(true) => {
                    processed += 1;
                    if config.once {
                        break;
                    }
                }
                Ok(false) => {}
                Err(err) => eprintln!("跳过归档会话 {}: {err:#}", session.display()),
            }
        }
        return Ok(processed);
    }

    let mut state = WatchState::load(&config.state_path)?;
    let mut changed = false;
    for session in sessions {
        match snapshot_and_record(&session) {
            Ok((_snapshot, record)) => {
                if !state.contains(&record) {
                    state.insert(record);
                    changed = true;
                }
            }
            Err(err) => eprintln!("跳过归档会话 {}: {err:#}", session.display()),
        }
    }
    if changed {
        state.save(&config.state_path)?;
    }
    Ok(0)
}

/// 处理单个归档会话文件。
///
/// 成功生成并通知新会话时返回 `true`；如果状态文件显示已经处理过，返回 `false`。解析或写入失败时返回错误，
/// 且不会写入已处理状态，便于后续重试。
pub fn process_archived_session(
    path: &Path,
    config: &WatchConfig,
    notifier: &dyn Notifier,
    opener: &dyn Opener,
) -> Result<bool> {
    if !is_jsonl(path) {
        return Ok(false);
    }

    let (snapshot, record) = snapshot_and_record(path)?;
    let mut state = WatchState::load(&config.state_path)?;
    if state.contains(&record) {
        return Ok(false);
    }

    fs::create_dir_all(&config.receipt_dir)
        .with_context(|| format!("无法创建小票目录 {}", config.receipt_dir.display()))?;
    let receipt_path = receipt_path(&config.receipt_dir, &snapshot);
    let html = render_html_receipt(&snapshot, &config.pricing_path)?;
    fs::write(&receipt_path, format!("{html}\n"))
        .with_context(|| format!("无法写入 HTML 小票 {}", receipt_path.display()))?;

    let message = notification_message(&snapshot, &receipt_path, &config.pricing_path)?;
    if let Err(err) = notifier.notify(&message) {
        eprintln!(
            "系统通知失败，但 HTML 小票已生成: {} ({err:#})",
            receipt_path.display()
        );
    }
    if config.open_receipt {
        if let Err(err) = opener.open(&receipt_path) {
            eprintln!(
                "打开 HTML 小票失败，但文件已生成: {} ({err:#})",
                receipt_path.display()
            );
        }
    }

    state.insert(record);
    state.save(&config.state_path)?;
    Ok(true)
}

/// 常驻监听归档目录并处理新出现的会话文件。
///
/// `once` 为真时，处理到第一个新归档会话后退出；否则会持续运行直到进程被终止。
pub fn run_watch(config: WatchConfig, notifier: &dyn Notifier, opener: &dyn Opener) -> Result<()> {
    fs::create_dir_all(&config.archived_dir)
        .with_context(|| format!("无法创建归档监听目录 {}", config.archived_dir.display()))?;
    fs::create_dir_all(&config.receipt_dir)
        .with_context(|| format!("无法创建小票目录 {}", config.receipt_dir.display()))?;

    let initial_processed = run_initial_scan(&config, notifier, opener)?;
    if config.once && initial_processed > 0 {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx)?;
    watcher.watch(&config.archived_dir, RecursiveMode::Recursive)?;

    for event in rx {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                eprintln!("监听 Codex 归档目录失败: {err:#}");
                continue;
            }
        };

        for path in event.paths.into_iter().filter(|path| is_jsonl(path)) {
            std::thread::sleep(Duration::from_millis(500));
            match process_archived_session(&path, &config, notifier, opener) {
                Ok(true) if config.once => return Ok(()),
                Ok(_) => {}
                Err(err) => eprintln!("处理归档会话 {} 失败: {err:#}", path.display()),
            }
        }
    }

    Ok(())
}

/// 执行一次通知和打开链路测试。
///
/// 该函数会生成一份固定内容的 HTML 测试小票，然后尝试发送系统通知；`open_receipt` 为真时继续尝试打开该文件。
/// 通知或打开失败会记录为报告状态，不会让命令失败。
pub fn run_notify_test(
    receipt_dir: &Path,
    open_receipt: bool,
    notifier: &dyn Notifier,
    opener: &dyn Opener,
) -> Result<NotifyTestReport> {
    fs::create_dir_all(receipt_dir)
        .with_context(|| format!("无法创建小票目录 {}", receipt_dir.display()))?;
    let snapshot = notify_test_snapshot(receipt_dir);
    let receipt_path = receipt_path(receipt_dir, &snapshot);
    let estimate =
        crate::pricing::estimate_cost(&snapshot, crate::pricing::default_pricing_path())?;
    let view = crate::receipt::build_receipt_view(&snapshot, &estimate, 48);
    let html = crate::render_html::render_html(&view);
    fs::write(&receipt_path, format!("{html}\n"))
        .with_context(|| format!("无法写入测试小票 {}", receipt_path.display()))?;

    let message = NotificationMessage {
        title: "Codex 小票通知测试".to_string(),
        body: format!("通知测试\n{}\n{}", view.total_row.1, receipt_path.display()),
    };
    let notification_status = match notifier.notify(&message) {
        Ok(()) => "attempted".to_string(),
        Err(err) => {
            eprintln!("系统通知测试失败: {err:#}");
            format!("failed: {err}")
        }
    };

    let open_status = if open_receipt {
        match opener.open(&receipt_path) {
            Ok(()) => "attempted".to_string(),
            Err(err) => {
                eprintln!("打开测试小票失败: {err:#}");
                format!("failed: {err}")
            }
        }
    } else {
        "skipped".to_string()
    };

    Ok(NotifyTestReport {
        receipt_path,
        notification_status,
        open_status,
    })
}

fn render_html_receipt(snapshot: &UsageSnapshot, pricing_path: &Path) -> Result<String> {
    let estimate = crate::pricing::estimate_cost(snapshot, pricing_path)?;
    let view = crate::receipt::build_receipt_view(snapshot, &estimate, 48);
    Ok(crate::render_html::render_html(&view))
}

fn notification_message(
    snapshot: &UsageSnapshot,
    receipt_path: &Path,
    pricing_path: &Path,
) -> Result<NotificationMessage> {
    let estimate = crate::pricing::estimate_cost(snapshot, pricing_path)?;
    let view = crate::receipt::build_receipt_view(snapshot, &estimate, 48);
    let price = view
        .pricing_rows
        .first()
        .map(|(_label, value)| value.as_str())
        .unwrap_or("价格未映射");
    Ok(NotificationMessage {
        title: "Codex 小票已生成".to_string(),
        body: format!(
            "{}\n{}\n{}\n{}",
            snapshot.model,
            view.total_row.1,
            price,
            receipt_path.display()
        ),
    })
}

fn snapshot_and_record(path: &Path) -> Result<(UsageSnapshot, ProcessedSession)> {
    let snapshot = crate::codex_logs::load_snapshot_from_session(path, Scope::Session, None, None)?;
    let modified_ms = modified_ms(path);
    let key = format!(
        "{}|{}|{}|{}",
        snapshot.session_id,
        path.to_string_lossy(),
        snapshot.timestamp.as_deref().unwrap_or(""),
        modified_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    let record = ProcessedSession {
        key,
        session_id: snapshot.session_id.clone(),
        source: path.to_path_buf(),
        token_timestamp: snapshot.timestamp.clone(),
        modified_ms,
    };
    Ok((snapshot, record))
}

fn receipt_path(receipt_dir: &Path, snapshot: &UsageSnapshot) -> PathBuf {
    let timestamp = snapshot
        .timestamp
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    receipt_dir.join(format!(
        "codex-receipt-{}-{}.html",
        timestamp,
        short_session_id(&snapshot.session_id)
    ))
}

fn short_session_id(session_id: &str) -> String {
    let mut out = session_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(24)
        .collect::<String>();
    if out.is_empty() {
        out = "session".to_string();
    }
    out
}

fn archived_session_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_jsonl(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_jsonl(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root).with_context(|| format!("无法读取 {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, out)?;
        } else if is_jsonl(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_jsonl(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
}

fn modified_ms(path: &Path) -> Option<i64> {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_time_ms)
}

fn system_time_ms(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn notify_test_snapshot(receipt_dir: &Path) -> UsageSnapshot {
    UsageSnapshot {
        input_tokens: 1_024,
        context_input_tokens: 1_024,
        cached_input_tokens: 256,
        output_tokens: 128,
        reasoning_output_tokens: 0,
        total_tokens: 1_152,
        context_window: Some(128_000),
        provider: "openai".to_string(),
        model: "通知测试".to_string(),
        source: receipt_dir.join("notify-test.jsonl"),
        session_id: "notify-test".to_string(),
        timestamp: Some(chrono::Local::now().to_rfc3339()),
        scope: "session".to_string(),
        available_fields: vec![
            "cached_input_tokens".to_string(),
            "input_tokens".to_string(),
            "output_tokens".to_string(),
            "total_tokens".to_string(),
        ],
    }
}
