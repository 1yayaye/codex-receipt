use anyhow::anyhow;
use codex_receipt::desktop_notify::{NotificationMessage, Notifier};
use codex_receipt::opener::Opener;
use codex_receipt::watch::{
    process_archived_session, run_initial_scan, run_notify_test, WatchConfig,
};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct FakeNotifier {
    messages: RefCell<Vec<NotificationMessage>>,
    fail: bool,
}

impl Notifier for FakeNotifier {
    fn notify(&self, message: &NotificationMessage) -> anyhow::Result<()> {
        if self.fail {
            return Err(anyhow!("notification failed"));
        }
        self.messages.borrow_mut().push(message.clone());
        Ok(())
    }
}

#[derive(Default)]
struct FakeOpener {
    paths: RefCell<Vec<PathBuf>>,
    fail: bool,
}

impl Opener for FakeOpener {
    fn open(&self, path: &Path) -> anyhow::Result<()> {
        if self.fail {
            return Err(anyhow!("open failed"));
        }
        self.paths.borrow_mut().push(path.to_path_buf());
        Ok(())
    }
}

#[test]
fn baseline_existing_archives_without_replay() {
    let env = WatchEnv::new();
    let session = env.write_session(
        "existing-session",
        "2026-05-04T01:00:00Z",
        25_000,
        "gpt-5.4",
    );
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();

    let processed = run_initial_scan(&env.config(false), &notifier, &opener).unwrap();

    assert_eq!(processed, 0);
    assert!(env.state_text().contains("existing-session"));
    assert!(env.receipts().is_empty());
    assert!(notifier.messages.borrow().is_empty());
    assert!(opener.paths.borrow().is_empty());
    assert!(session.exists());
}

#[test]
fn replay_existing_generates_html_and_notification() {
    let env = WatchEnv::new();
    env.write_session("replay-session", "2026-05-04T01:00:00Z", 25_000, "gpt-5.4");
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();

    let processed = run_initial_scan(&env.config(true), &notifier, &opener).unwrap();

    assert_eq!(processed, 1);
    let receipts = env.receipts();
    assert_eq!(receipts.len(), 1);
    let html = fs::read_to_string(&receipts[0]).unwrap();
    assert!(html.contains("lang=\"zh-CN\""));
    assert!(html.contains("打印小票"));
    assert!(html.contains("25,000 Tokens"));
    assert_eq!(notifier.messages.borrow().len(), 1);
    let message = notifier.messages.borrow()[0].clone();
    assert_eq!(message.title, "Codex 小票已生成");
    assert!(message.body.contains("gpt-5.4"));
    assert!(message.body.contains("25,000 Tokens"));
    assert!(message
        .body
        .contains(receipts[0].to_string_lossy().as_ref()));
    assert_eq!(opener.paths.borrow().as_slice(), receipts.as_slice());
}

#[test]
fn processed_archived_session_is_not_notified_twice() {
    let env = WatchEnv::new();
    let session = env.write_session("dedupe-session", "2026-05-04T01:00:00Z", 25_000, "gpt-5.4");
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();
    let config = env.config(true);

    assert!(process_archived_session(&session, &config, &notifier, &opener).unwrap());
    assert!(!process_archived_session(&session, &config, &notifier, &opener).unwrap());

    assert_eq!(env.receipts().len(), 1);
    assert_eq!(notifier.messages.borrow().len(), 1);
    assert_eq!(opener.paths.borrow().len(), 1);
}

#[test]
fn invalid_archived_session_is_not_recorded() {
    let env = WatchEnv::new();
    let bad = env.archived_dir.join("bad.jsonl");
    fs::write(&bad, "{\"type\":\"session_meta\"}\n").unwrap();
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();

    let err = process_archived_session(&bad, &env.config(true), &notifier, &opener).unwrap_err();

    assert!(err.to_string().contains("token_count"));
    assert!(!env.state_path.exists());
    assert!(env.receipts().is_empty());
    assert!(notifier.messages.borrow().is_empty());
    assert!(opener.paths.borrow().is_empty());
}

#[test]
fn no_open_keeps_receipt_file_but_does_not_open_it() {
    let env = WatchEnv::new();
    let session = env.write_session("no-open-session", "2026-05-04T01:00:00Z", 25_000, "gpt-5.4");
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();
    let mut config = env.config(true);
    config.open_receipt = false;

    assert!(process_archived_session(&session, &config, &notifier, &opener).unwrap());

    assert_eq!(env.receipts().len(), 1);
    assert_eq!(notifier.messages.borrow().len(), 1);
    assert!(opener.paths.borrow().is_empty());
    assert!(env.state_text().contains("no-open-session"));
}

#[test]
fn notification_failure_does_not_block_open_or_state() {
    let env = WatchEnv::new();
    let session = env.write_session("notify-fails", "2026-05-04T01:00:00Z", 25_000, "gpt-5.4");
    let notifier = FakeNotifier {
        messages: RefCell::default(),
        fail: true,
    };
    let opener = FakeOpener::default();

    assert!(process_archived_session(&session, &env.config(true), &notifier, &opener).unwrap());

    assert_eq!(env.receipts().len(), 1);
    assert_eq!(opener.paths.borrow().len(), 1);
    assert!(env.state_text().contains("notify-fails"));
}

#[test]
fn open_failure_does_not_block_notification_or_state() {
    let env = WatchEnv::new();
    let session = env.write_session("open-fails", "2026-05-04T01:00:00Z", 25_000, "gpt-5.4");
    let notifier = FakeNotifier::default();
    let opener = FakeOpener {
        paths: RefCell::default(),
        fail: true,
    };

    assert!(process_archived_session(&session, &env.config(true), &notifier, &opener).unwrap());

    assert_eq!(env.receipts().len(), 1);
    assert_eq!(notifier.messages.borrow().len(), 1);
    assert!(env.state_text().contains("open-fails"));
}

#[test]
fn notify_test_uses_notifier_and_opener() {
    let env = WatchEnv::new();
    let notifier = FakeNotifier::default();
    let opener = FakeOpener::default();

    let report = run_notify_test(&env.receipt_dir, true, &notifier, &opener).unwrap();

    assert!(report.receipt_path.exists());
    assert_eq!(report.notification_status, "attempted");
    assert_eq!(report.open_status, "attempted");
    assert_eq!(notifier.messages.borrow().len(), 1);
    assert_eq!(
        opener.paths.borrow().as_slice(),
        std::slice::from_ref(&report.receipt_path)
    );
}

struct WatchEnv {
    _dir: tempfile::TempDir,
    archived_dir: PathBuf,
    receipt_dir: PathBuf,
    state_path: PathBuf,
}

impl WatchEnv {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let archived_dir = dir.path().join(".codex").join("archived_sessions");
        let receipt_dir = dir.path().join("receipts");
        let state_path = dir.path().join("state.json");
        fs::create_dir_all(&archived_dir).unwrap();
        fs::create_dir_all(&receipt_dir).unwrap();
        Self {
            _dir: dir,
            archived_dir,
            receipt_dir,
            state_path,
        }
    }

    fn config(&self, replay_existing: bool) -> WatchConfig {
        WatchConfig {
            archived_dir: self.archived_dir.clone(),
            receipt_dir: self.receipt_dir.clone(),
            state_path: self.state_path.clone(),
            pricing_path: PathBuf::from("references/pricing.json"),
            replay_existing,
            once: false,
            open_receipt: true,
        }
    }

    fn write_session(&self, id: &str, timestamp: &str, total_tokens: u64, model: &str) -> PathBuf {
        let path = self.archived_dir.join(format!("{id}.jsonl"));
        fs::write(&path, session_jsonl(id, timestamp, total_tokens, model)).unwrap();
        path
    }

    fn receipts(&self) -> Vec<PathBuf> {
        let mut receipts = read_dir_paths(&self.receipt_dir);
        receipts.sort();
        receipts
    }

    fn state_text(&self) -> String {
        fs::read_to_string(&self.state_path).unwrap()
    }
}

fn read_dir_paths(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect()
}

fn session_jsonl(id: &str, timestamp: &str, total_tokens: u64, model: &str) -> String {
    let lines = [
        serde_json::json!({
            "timestamp": timestamp,
            "type": "session_meta",
            "payload": {
                "id": id,
                "timestamp": timestamp,
                "model_provider": "openai"
            }
        }),
        serde_json::json!({
            "timestamp": timestamp,
            "type": "turn_context",
            "payload": {
                "model": model
            }
        }),
        serde_json::json!({
            "timestamp": timestamp,
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "last_token_usage": {
                        "input_tokens": total_tokens,
                        "output_tokens": 0,
                        "total_tokens": total_tokens
                    },
                    "total_token_usage": {
                        "input_tokens": total_tokens,
                        "output_tokens": 0,
                        "total_tokens": total_tokens
                    }
                }
            }
        }),
    ];

    format!(
        "{}\n",
        lines
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    )
}
