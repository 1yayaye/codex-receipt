use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use unicode_width::UnicodeWidthStr;

#[test]
fn show_fields_reports_fixture_usage() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--show-fields",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"input_tokens\": 20000"))
        .stdout(predicate::str::contains("\"context_input_tokens\": 12487"))
        .stdout(predicate::str::contains("\"scope\": \"session\""))
        .stdout(predicate::str::contains("\"model\": \"gpt-5.4\""))
        .stdout(predicate::str::contains("cached_input_tokens"));
}

#[test]
fn latest_turn_scope_uses_last_token_usage() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--scope",
        "latest-turn",
        "--show-fields",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"input_tokens\": 12487"))
        .stdout(predicate::str::contains("\"context_input_tokens\": 12487"))
        .stdout(predicate::str::contains("\"total_tokens\": 15702"))
        .stdout(predicate::str::contains("\"scope\": \"latest-turn\""));
}

#[test]
fn renders_chinese_receipt_from_fixture() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args(["--session", "tests/fixtures/codex-session.jsonl"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("CODEX"))
        .stdout(predicate::str::contains("感谢使用 Codex"))
        .stdout(predicate::str::contains("输入 Tokens"))
        .stdout(predicate::str::contains("缓存读取"))
        .stdout(predicate::str::contains("推理 Tokens"))
        .stdout(predicate::str::contains("12,487/258,400"))
        .stdout(predicate::str::contains("20,000/258,400").not())
        .stdout(predicate::str::contains("25,000 Tokens"))
        .stdout(predicate::str::contains("USD 预估"))
        .stdout(predicate::str::contains("推理不免费"));
}

#[test]
fn writes_printable_html_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receipt.html");
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--output",
        "html",
        "--write",
        path.to_str().unwrap(),
    ]);

    cmd.assert().success().stdout(predicate::str::is_empty());
    let html = std::fs::read_to_string(path).unwrap();
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("lang=\"zh-CN\""));
    assert!(html.contains("打印小票"));
    assert!(html.contains("USD 预估"));
}

#[test]
fn receipt_lines_do_not_exceed_width() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    let output = cmd
        .args([
            "--session",
            "tests/fixtures/codex-session.jsonl",
            "--width",
            "42",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for line in text.lines() {
        let width = UnicodeWidthStr::width(line);
        assert!(width <= 42, "line too wide: {width} > 42: {line:?}");
    }
}

#[test]
fn session_scope_uses_total_token_usage() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--scope",
        "session",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("25,000 Tokens"));
}

#[test]
fn custom_provider_gpt55_uses_official_model_price() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--provider",
        "custom",
        "--model",
        "gpt-5.5",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("CUSTOM"))
        .stdout(predicate::str::contains("gpt-5.5"))
        .stdout(predicate::str::contains("$0.205000"))
        .stdout(predicate::str::contains("价格映射"))
        .stdout(predicate::str::contains("按 OpenAI API 标准价估算"));
}

#[test]
fn unknown_model_renders_unmapped_price() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session-missing-model.jsonl",
        "--model",
        "unknown-model",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("unknown-model"))
        .stdout(predicate::str::contains("价格未映射"));
}

#[test]
fn write_suppresses_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receipt.txt");
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "--session",
        "tests/fixtures/codex-session.jsonl",
        "--write",
        path.to_str().unwrap(),
    ]);

    cmd.assert().success().stdout(predicate::str::is_empty());
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("CODEX"));
}

#[test]
fn watch_help_lists_background_options() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args(["watch", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--receipt-dir"))
        .stdout(predicate::str::contains("--state"))
        .stdout(predicate::str::contains("--replay-existing"))
        .stdout(predicate::str::contains("--no-open"))
        .stdout(predicate::str::contains("--once"));
}

#[test]
fn notify_test_no_open_writes_test_receipt_and_reports_attempts() {
    let dir = tempfile::tempdir().unwrap();
    let receipt_dir = dir.path().join("receipts");
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.args([
        "notify-test",
        "--no-open",
        "--receipt-dir",
        receipt_dir.to_str().unwrap(),
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("测试小票:"))
        .stdout(predicate::str::contains("系统通知:"))
        .stdout(predicate::str::contains("打开小票: skipped"));

    let receipts = fs::read_dir(receipt_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 1);
    let html = fs::read_to_string(&receipts[0]).unwrap();
    assert!(html.contains("lang=\"zh-CN\""));
    assert!(html.contains("通知测试"));
}

#[test]
fn default_session_skips_current_codex_thread() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir
        .path()
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("05")
        .join("04");
    fs::create_dir_all(&sessions).unwrap();

    fs::write(
        sessions.join("rollout-2026-05-04T19-00-00-current-thread-id.jsonl"),
        session_jsonl(
            "current-thread-id",
            "2026-05-04T11:45:00Z",
            45_000,
            "gpt-current",
        ),
    )
    .unwrap();
    fs::write(
        sessions.join("rollout-2026-05-04T18-50-00-older-id.jsonl"),
        session_jsonl("older-id", "2026-05-04T11:01:50Z", 10_000, "gpt-older"),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    cmd.env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .env("CODEX_THREAD_ID", "current-thread-id")
        .arg("--show-fields");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"session_id\": \"older-id\""))
        .stdout(predicate::str::contains("\"model\": \"gpt-older\""))
        .stdout(predicate::str::contains("\"total_tokens\": 10000"));
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
