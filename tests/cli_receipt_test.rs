use assert_cmd::Command;
use predicates::prelude::*;
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
        .stdout(predicate::str::contains("\"input_tokens\": 12487"))
        .stdout(predicate::str::contains("\"model\": \"gpt-5.4\""))
        .stdout(predicate::str::contains("cached_input_tokens"));
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
