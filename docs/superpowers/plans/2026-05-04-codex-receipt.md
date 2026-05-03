# Codex 中文 Token 小票 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI that reads local Codex session logs and prints a Chinese, screenshot-ready token usage receipt.

**Architecture:** The CLI is a small Rust crate with separate modules for Codex log discovery/parsing, price estimation, receipt view modeling, text rendering, HTML rendering, and command-line orchestration. v1 is Codex-only by design, with honest fallbacks for missing fields and unmapped model pricing.

**Tech Stack:** Rust 2021/2024 edition, `clap` for CLI parsing, `serde`/`serde_json` for JSONL and pricing data, `chrono` for dates, `unicode-width` for Chinese terminal alignment, `assert_cmd`/`predicates`/`tempfile` for CLI tests.

---

## Summary

This project should not become another AI usage dashboard. Existing tools such as ccusage, Tokscale, TokenTelemetry, Costea, and AI Observer already cover analytics, historical reports, dashboards, and multi-agent monitoring.

The wedge is narrower:

- Codex first.
- Chinese receipt feel first.
- Chat output first.
- Rust binary distribution first.

The first release should make one thing feel excellent: run one command, get a Chinese token receipt that is accurate enough to trust and distinctive enough to share.

## Public Interface

Command name:

```bash
codex-receipt
```

Required v1 commands:

```bash
codex-receipt
codex-receipt --scope session
codex-receipt --session path/to/session.jsonl
codex-receipt --width 48
codex-receipt --output html --write receipt.html
codex-receipt --show-fields
```

CLI options:

```text
--session <PATH>              Use a specific Codex JSONL session file.
--scope <latest-turn|session> Select last turn or full-session token usage. Default: latest-turn.
--width <42|48|56|64>         Receipt text width. Default: 48.
--output <text|html>          Render text receipt or printable HTML. Default: text.
--write <PATH>                Write output to file and suppress stdout.
--pricing <PATH>              Use an alternate pricing JSON file.
--model <MODEL>               Override model display and pricing lookup.
--provider <PROVIDER>         Override provider display and pricing lookup.
--show-fields                 Print machine-readable field availability JSON.
```

## File Structure

Create these files:

```text
Cargo.toml
src/main.rs
src/cli.rs
src/codex_logs.rs
src/models.rs
src/pricing.rs
src/receipt.rs
src/render_text.rs
src/render_html.rs
references/pricing.json
tests/fixtures/codex-session.jsonl
tests/fixtures/codex-session-missing-model.jsonl
tests/cli_receipt_test.rs
README.md
```

Responsibilities:

- `src/main.rs`: thin entrypoint that calls `cli::run()`.
- `src/cli.rs`: parses arguments, selects source, calls rendering, handles `--write` and stdout behavior.
- `src/codex_logs.rs`: finds Codex session files, parses JSONL, extracts `session_meta`, `turn_context`, and `token_count` events.
- `src/models.rs`: shared structs and formatting helpers.
- `src/pricing.rs`: loads pricing table, matches provider/model/aliases, estimates cost, returns unmapped fallback.
- `src/receipt.rs`: converts a usage snapshot and price estimate into receipt rows.
- `src/render_text.rs`: fixed-width Chinese text receipt renderer.
- `src/render_html.rs`: printable receipt HTML renderer.
- `references/pricing.json`: static v1 price table with source metadata.
- `tests/fixtures/*`: stable Codex JSONL examples.
- `tests/cli_receipt_test.rs`: end-to-end CLI tests.
- `README.md`: install, usage, philosophy, limitations, validation.

## Task 1: Scaffold Rust Crate

**Files:**

- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/models.rs`

- [ ] **Step 1: Create the package manifest**

Add `Cargo.toml`:

```toml
[package]
name = "codex-receipt"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Print Codex token usage as a Chinese receipt"
repository = "https://github.com/OWNER/codex-receipt"

[[bin]]
name = "codex-receipt"
path = "src/main.rs"

[dependencies]
anyhow = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
unicode-width = "0.2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 2: Add the thin entrypoint**

Add `src/main.rs`:

```rust
mod cli;
mod codex_logs;
mod models;
mod pricing;
mod receipt;
mod render_html;
mod render_text;

fn main() -> anyhow::Result<()> {
    cli::run()
}
```

- [ ] **Step 3: Add initial shared models**

Add `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    LatestTurn,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Html,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSnapshot {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
    pub context_window: Option<u64>,
    pub provider: String,
    pub model: String,
    pub source: PathBuf,
    pub session_id: String,
    pub timestamp: Option<String>,
    pub scope: String,
    pub available_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PriceEstimate {
    pub status: PriceStatus,
    pub amount: Option<f64>,
    pub currency: String,
    pub model: String,
    pub source_checked_at: Option<String>,
    pub rate_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceStatus {
    Estimate,
    Unmapped,
}

#[derive(Debug, Deserialize)]
pub struct PricingTable {
    pub checked_at: Option<String>,
    pub currency: Option<String>,
    pub unit: Option<String>,
    pub models: Vec<PricingEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PricingEntry {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub input_per_million: f64,
    #[serde(default)]
    pub cached_input_per_million: Option<f64>,
    pub output_per_million: f64,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub source_checked_at: Option<String>,
    #[serde(default)]
    pub rate_note: Option<String>,
}

pub fn fmt_int(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();
    for (idx, ch) in text.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}
```

- [ ] **Step 4: Add CLI skeleton**

Add `src/cli.rs`:

```rust
use crate::models::{OutputFormat, Scope};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "codex-receipt")]
#[command(about = "Print Codex token usage as a Chinese receipt")]
pub struct Args {
    #[arg(long)]
    pub session: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = ScopeArg::LatestTurn)]
    pub scope: ScopeArg,

    #[arg(long, default_value_t = 48, value_parser = [42, 48, 56, 64])]
    pub width: usize,

    #[arg(long, value_enum, default_value_t = OutputArg::Text)]
    pub output: OutputArg,

    #[arg(long)]
    pub write: Option<PathBuf>,

    #[arg(long)]
    pub pricing: Option<PathBuf>,

    #[arg(long)]
    pub model: Option<String>,

    #[arg(long)]
    pub provider: Option<String>,

    #[arg(long)]
    pub show_fields: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ScopeArg {
    LatestTurn,
    Session,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputArg {
    Text,
    Html,
}

impl From<ScopeArg> for Scope {
    fn from(value: ScopeArg) -> Self {
        match value {
            ScopeArg::LatestTurn => Scope::LatestTurn,
            ScopeArg::Session => Scope::Session,
        }
    }
}

impl From<OutputArg> for OutputFormat {
    fn from(value: OutputArg) -> Self {
        match value {
            OutputArg::Text => OutputFormat::Text,
            OutputArg::Html => OutputFormat::Html,
        }
    }
}

pub fn run() -> Result<()> {
    let _args = Args::parse();
    anyhow::bail!("implementation not wired yet")
}
```

- [ ] **Step 5: Run compile check and verify expected incomplete error**

Run:

```bash
cargo check
```

Expected: compile succeeds. Running `cargo run --` should fail with `implementation not wired yet`.

## Task 2: Parse Codex JSONL Sessions

**Files:**

- Create: `src/codex_logs.rs`
- Create: `tests/fixtures/codex-session.jsonl`
- Create: `tests/fixtures/codex-session-missing-model.jsonl`
- Modify: `src/cli.rs`

- [ ] **Step 1: Add fixture with token_count data**

Add `tests/fixtures/codex-session.jsonl`:

```jsonl
{"timestamp":"2026-05-04T01:00:00Z","type":"session_meta","payload":{"id":"fixture-session","timestamp":"2026-05-04T00:58:00Z","model_provider":"openai"}}
{"timestamp":"2026-05-04T01:00:01Z","type":"turn_context","payload":{"model":"gpt-5.4"}}
{"timestamp":"2026-05-04T01:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"model_context_window":258400,"last_token_usage":{"input_tokens":12487,"cached_input_tokens":8742,"output_tokens":3215,"reasoning_output_tokens":128,"total_tokens":15702},"total_token_usage":{"input_tokens":20000,"cached_input_tokens":10000,"output_tokens":5000,"reasoning_output_tokens":256,"total_tokens":25000}}}}
```

Add `tests/fixtures/codex-session-missing-model.jsonl`:

```jsonl
{"timestamp":"2026-05-04T01:00:00Z","type":"session_meta","payload":{"id":"missing-model","timestamp":"2026-05-04T00:58:00Z","model_provider":"openai"}}
{"timestamp":"2026-05-04T01:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":20,"total_tokens":120}}}}
```

- [ ] **Step 2: Implement JSONL parser**

Add `src/codex_logs.rs`:

```rust
use crate::models::{Scope, UsageSnapshot};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub fn newest_session_file() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Could not resolve home directory"))?;

    let mut candidates = Vec::new();
    for root in [
        home.join(".codex").join("sessions"),
        home.join(".codex").join("archived_sessions"),
    ] {
        collect_jsonl_files(&root, &mut candidates)?;
    }

    candidates
        .into_iter()
        .max_by_key(|path| path.metadata().and_then(|m| m.modified()).ok())
        .ok_or_else(|| anyhow!("No Codex session file found under ~/.codex/sessions or ~/.codex/archived_sessions"))
}

fn collect_jsonl_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(root).with_context(|| format!("Failed to read {}", root.display()))? {
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

pub fn load_snapshot_from_session(
    path: &Path,
    scope: Scope,
    model_override: Option<&str>,
    provider_override: Option<&str>,
) -> Result<UsageSnapshot> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
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
        let payload = item.get("payload").unwrap_or(&Value::Null);

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
            model = payload.get("model").and_then(Value::as_str).map(str::to_string);
        }

        if item_type == "event_msg" && payload.get("type").and_then(Value::as_str) == Some("token_count") {
            token_info = payload.get("info").cloned();
            timestamp = item.get("timestamp").and_then(Value::as_str).map(str::to_string);
        }
    }

    let info = token_info.ok_or_else(|| anyhow!("No token_count event found in {}", path.display()))?;
    let usage_key = match scope {
        Scope::LatestTurn => "last_token_usage",
        Scope::Session => "total_token_usage",
    };
    let usage = info
        .get(usage_key)
        .ok_or_else(|| anyhow!("No {} found in token_count event", usage_key))?;

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
    let total_tokens = u64_field(usage, "total_tokens").unwrap_or(input_tokens + output_tokens);

    Ok(UsageSnapshot {
        input_tokens,
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

fn u64_field(value: &Value, key: &str) -> u64 {
    u64_field_opt(value, key).unwrap_or(0)
}

fn u64_field_opt(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}
```

- [ ] **Step 3: Wire parser into CLI for `--show-fields`**

Replace `run()` in `src/cli.rs` with:

```rust
pub fn run() -> Result<()> {
    let args = Args::parse();
    let session_path = match args.session.as_ref() {
        Some(path) => path.clone(),
        None => crate::codex_logs::newest_session_file()?,
    };
    let snapshot = crate::codex_logs::load_snapshot_from_session(
        &session_path,
        args.scope.into(),
        args.model.as_deref(),
        args.provider.as_deref(),
    )?;

    if args.show_fields {
        let json = serde_json::to_string_pretty(&snapshot)?;
        if let Some(path) = args.write {
            std::fs::write(path, format!("{json}\n"))?;
        } else {
            println!("{json}");
        }
        return Ok(());
    }

    anyhow::bail!("receipt rendering not wired yet")
}
```

- [ ] **Step 4: Add parser tests through CLI**

Add the first test to `tests/cli_receipt_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

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
```

- [ ] **Step 5: Run the test**

Run:

```bash
cargo test show_fields_reports_fixture_usage
```

Expected: PASS.

## Task 3: Implement Pricing

**Files:**

- Create: `src/pricing.rs`
- Create: `references/pricing.json`
- Modify: `src/cli.rs`
- Modify: `tests/cli_receipt_test.rs`

- [ ] **Step 1: Add minimal v1 pricing table**

Add `references/pricing.json`:

```json
{
  "checked_at": "2026-05-04",
  "currency": "USD",
  "unit": "per_1m_tokens",
  "models": [
    {
      "provider": "openai",
      "model": "gpt-5.4",
      "aliases": ["gpt-5.4", "gpt 5.4"],
      "input_per_million": 2.5,
      "cached_input_per_million": 0.25,
      "output_per_million": 15.0,
      "source_url": "https://openai.com/api/pricing/",
      "source_checked_at": "2026-05-04"
    },
    {
      "provider": "openai",
      "model": "gpt-5.4-mini",
      "aliases": ["gpt-5.4-mini", "gpt 5.4 mini"],
      "input_per_million": 0.75,
      "cached_input_per_million": 0.075,
      "output_per_million": 4.5,
      "source_url": "https://openai.com/api/pricing/",
      "source_checked_at": "2026-05-04"
    }
  ]
}
```

- [ ] **Step 2: Implement price lookup and estimation**

Add `src/pricing.rs`:

```rust
use crate::models::{normalize_key, PriceEstimate, PriceStatus, PricingEntry, PricingTable, UsageSnapshot};
use anyhow::{Context, Result};
use std::path::Path;

pub fn default_pricing_path() -> &'static Path {
    Path::new("references/pricing.json")
}

pub fn estimate_cost(snapshot: &UsageSnapshot, pricing_path: &Path) -> Result<PriceEstimate> {
    let table = load_pricing(pricing_path)?;
    let Some(entry) = find_price(&table, &snapshot.provider, &snapshot.model) else {
        return Ok(PriceEstimate {
            status: PriceStatus::Unmapped,
            amount: None,
            currency: table.currency.unwrap_or_else(|| "USD".to_string()),
            model: "价格未映射".to_string(),
            source_checked_at: table.checked_at,
            rate_note: None,
        });
    };

    let cached = snapshot.cached_input_tokens.min(snapshot.input_tokens);
    let uncached = snapshot.input_tokens.saturating_sub(cached);
    let input_rate = entry.input_per_million;
    let cached_rate = entry.cached_input_per_million.unwrap_or(input_rate);
    let output_rate = entry.output_per_million;
    let amount = ((uncached as f64) * input_rate
        + (cached as f64) * cached_rate
        + (snapshot.output_tokens as f64) * output_rate)
        / 1_000_000.0;

    Ok(PriceEstimate {
        status: PriceStatus::Estimate,
        amount: Some(amount),
        currency: entry
            .currency
            .clone()
            .or(table.currency)
            .unwrap_or_else(|| "USD".to_string())
            .to_uppercase(),
        model: entry.model.clone(),
        source_checked_at: entry.source_checked_at.clone().or(table.checked_at),
        rate_note: entry.rate_note.clone(),
    })
}

fn load_pricing(path: &Path) -> Result<PricingTable> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read pricing file {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("Failed to parse pricing file {}", path.display()))
}

fn find_price<'a>(table: &'a PricingTable, provider: &str, model: &str) -> Option<&'a PricingEntry> {
    let provider_key = normalize_key(provider);
    let model_key = normalize_key(model);

    table.models.iter().find(|entry| {
        let provider_matches =
            provider_key.is_empty() || provider_key == "unknown" || provider_key == normalize_key(&entry.provider);
        provider_matches && aliases_match(entry, &model_key)
    }).or_else(|| table.models.iter().find(|entry| aliases_match(entry, &model_key)))
}

fn aliases_match(entry: &PricingEntry, model_key: &str) -> bool {
    std::iter::once(&entry.model)
        .chain(entry.aliases.iter())
        .any(|alias| normalize_key(alias) == model_key)
}
```

- [ ] **Step 3: Load estimate in CLI**

In `src/cli.rs`, after snapshot creation, add:

```rust
    let pricing_path = args
        .pricing
        .as_deref()
        .unwrap_or_else(crate::pricing::default_pricing_path);
    let _estimate = crate::pricing::estimate_cost(&snapshot, pricing_path)?;
```

Keep the existing `receipt rendering not wired yet` error for now.

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test
```

Expected: PASS.

## Task 4: Build Receipt View and Text Renderer

**Files:**

- Create: `src/receipt.rs`
- Create: `src/render_text.rs`
- Modify: `src/cli.rs`
- Modify: `tests/cli_receipt_test.rs`

- [ ] **Step 1: Add receipt view model**

Add `src/receipt.rs`:

```rust
use crate::models::{fmt_int, PriceEstimate, PriceStatus, UsageSnapshot};
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct ReceiptView {
    pub width: usize,
    pub title: String,
    pub receipt_id: String,
    pub date: String,
    pub summary_rows: Vec<(String, String)>,
    pub token_rows: Vec<(String, String)>,
    pub total_row: (String, String),
    pub pricing_rows: Vec<(String, String)>,
    pub footer: String,
    pub barcode: String,
}

pub fn build_receipt_view(snapshot: &UsageSnapshot, estimate: &PriceEstimate, width: usize) -> ReceiptView {
    let receipt_id = receipt_id(snapshot);
    let mut token_rows = Vec::new();
    if snapshot.available_fields.iter().any(|f| f == "input_tokens") {
        token_rows.push(("输入 Tokens".to_string(), fmt_int(snapshot.input_tokens)));
    }
    if snapshot.available_fields.iter().any(|f| f == "output_tokens") {
        token_rows.push(("输出 Tokens".to_string(), fmt_int(snapshot.output_tokens)));
    }
    if snapshot.available_fields.iter().any(|f| f == "cached_input_tokens") {
        token_rows.push(("缓存读取".to_string(), fmt_int(snapshot.cached_input_tokens)));
    }
    if snapshot.available_fields.iter().any(|f| f == "reasoning_output_tokens") {
        token_rows.push(("推理 Tokens".to_string(), fmt_int(snapshot.reasoning_output_tokens)));
    }

    ReceiptView {
        width,
        title: "CODEX 小票".to_string(),
        receipt_id: receipt_id.clone(),
        date: display_time(snapshot.timestamp.as_deref()),
        summary_rows: vec![
            ("供应商".to_string(), snapshot.provider.to_uppercase()),
            ("模型".to_string(), snapshot.model.clone()),
            ("已用上下文".to_string(), context_used(snapshot)),
        ],
        token_rows,
        total_row: ("总计".to_string(), format!("{} Tokens", fmt_int(snapshot.total_tokens))),
        pricing_rows: pricing_rows(estimate),
        footer: choose_footer(snapshot),
        barcode: barcode(&receipt_id, width),
    }
}

fn receipt_id(snapshot: &UsageSnapshot) -> String {
    let seed = format!(
        "{}:{}:{}:{}",
        snapshot.session_id, snapshot.provider, snapshot.model, snapshot.total_tokens
    );
    let digest = stable_digest(&seed);
    format!("CX_{}_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), digest)
}

fn stable_digest(seed: &str) -> String {
    let mut hash: u32 = 2166136261;
    for byte in seed.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    format!("{:06X}", hash & 0xFF_FFFF)
}

fn display_time(value: Option<&str>) -> String {
    value
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
}

fn context_used(snapshot: &UsageSnapshot) -> String {
    match snapshot.context_window {
        Some(window) => format!("{}/{}", fmt_int(snapshot.input_tokens), fmt_int(window)),
        None => fmt_int(snapshot.input_tokens),
    }
}

fn pricing_rows(estimate: &PriceEstimate) -> Vec<(String, String)> {
    let label = format!("{} 预估", estimate.currency);
    let amount = match estimate.amount {
        Some(value) if estimate.currency == "USD" => format!("${value:.6}"),
        Some(value) if estimate.currency == "CNY" => format!("¥{value:.6}"),
        Some(value) => format!("{} {value:.6}", estimate.currency),
        None => "价格未映射".to_string(),
    };

    let mut rows = vec![(label, amount)];
    rows.push((
        "价格映射".to_string(),
        match estimate.status {
            PriceStatus::Estimate => estimate.model.clone(),
            PriceStatus::Unmapped => "价格未映射".to_string(),
        },
    ));
    if let Some(date) = &estimate.source_checked_at {
        rows.push(("价格日期".to_string(), date.clone()));
    }
    if let Some(note) = &estimate.rate_note {
        rows.push(("价格说明".to_string(), note.clone()));
    }
    rows
}

fn choose_footer(snapshot: &UsageSnapshot) -> String {
    if snapshot.reasoning_output_tokens > 0 {
        "推理不免费，犹豫更贵。".to_string()
    } else if snapshot.cached_input_tokens > 0 {
        "缓存省了一点，不够救你。".to_string()
    } else {
        "结果很体面，账单更诚实。".to_string()
    }
}

fn barcode(seed: &str, width: usize) -> String {
    let digest = stable_digest(seed);
    let mut bars = String::new();
    for ch in digest.chars().cycle().take(width.saturating_sub(16).max(24)) {
        match ch {
            '0'..='3' => bars.push('|'),
            '4'..='7' => bars.push_str("||"),
            '8'..='B' => bars.push_str("| "),
            _ => bars.push_str(" ||"),
        }
        if bars.len() >= width.saturating_sub(8) {
            break;
        }
    }
    bars
}
```

- [ ] **Step 2: Add fixed-width text renderer**

Add `src/render_text.rs`:

```rust
use crate::receipt::ReceiptView;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render_text(view: &ReceiptView) -> String {
    let mut out = Vec::new();
    center(&mut out, "█████", view.width);
    center(&mut out, "CODEX", view.width);
    out.push(String::new());
    center(&mut out, "感谢使用 Codex", view.width);
    center(&mut out, &format!("小票号: {}", view.receipt_id), view.width);
    center(&mut out, &format!("日期: {}", view.date), view.width);
    strong_rule(&mut out, view.width);
    for (left, right) in &view.summary_rows {
        kv(&mut out, left, right, view.width);
    }
    light_rule(&mut out, view.width);
    kv(&mut out, "项目", "TOKEN", view.width);
    light_rule(&mut out, view.width);
    for (left, right) in &view.token_rows {
        kv(&mut out, left, right, view.width);
    }
    strong_rule(&mut out, view.width);
    kv(&mut out, &view.total_row.0, &view.total_row.1, view.width);
    light_rule(&mut out, view.width);
    for (left, right) in &view.pricing_rows {
        kv(&mut out, left, right, view.width);
    }
    strong_rule(&mut out, view.width);
    center(&mut out, &view.footer, view.width);
    out.push(String::new());
    center(&mut out, &view.barcode, view.width);
    center(&mut out, &view.receipt_id, view.width);
    out.join("\n")
}

fn strong_rule(out: &mut Vec<String>, width: usize) {
    out.push("━".repeat(width));
}

fn light_rule(out: &mut Vec<String>, width: usize) {
    out.push("─".repeat(width));
}

fn center(out: &mut Vec<String>, text: &str, width: usize) {
    let text = truncate(text, width);
    let used = UnicodeWidthStr::width(text.as_str());
    let left = width.saturating_sub(used) / 2;
    out.push(format!("{}{}", " ".repeat(left), text));
}

fn kv(out: &mut Vec<String>, left: &str, right: &str, width: usize) {
    let right_width = UnicodeWidthStr::width(right);
    let max_left = width.saturating_sub(right_width + 1).max(1);
    let left = truncate(left, max_left);
    let left_width = UnicodeWidthStr::width(left.as_str());
    let spaces = width.saturating_sub(left_width + right_width).max(1);
    out.push(format!("{left}{}{right}", " ".repeat(spaces)));
}

fn truncate(text: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }
    let suffix = "...";
    let target = max_width.saturating_sub(UnicodeWidthStr::width(suffix));
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + width > target {
            break;
        }
        used += width;
        out.push(ch);
    }
    out.push_str(suffix);
    out
}
```

- [ ] **Step 3: Wire text rendering into CLI**

In `src/cli.rs`, replace the final incomplete error with:

```rust
    let view = crate::receipt::build_receipt_view(&snapshot, &_estimate, args.width);
    let rendered = match OutputFormat::from(args.output) {
        OutputFormat::Text => crate::render_text::render_text(&view),
        OutputFormat::Html => anyhow::bail!("html rendering not wired yet"),
    };

    if let Some(path) = args.write {
        std::fs::write(path, format!("{rendered}\n"))?;
    } else {
        println!("{rendered}");
    }
    Ok(())
```

- [ ] **Step 4: Add receipt output test**

Append to `tests/cli_receipt_test.rs`:

```rust
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
```

- [ ] **Step 5: Run receipt tests**

Run:

```bash
cargo test renders_chinese_receipt_from_fixture
```

Expected: PASS.

## Task 5: HTML Export

**Files:**

- Create: `src/render_html.rs`
- Modify: `src/cli.rs`
- Modify: `tests/cli_receipt_test.rs`

- [ ] **Step 1: Implement printable HTML renderer**

Add `src/render_html.rs`:

```rust
use crate::receipt::ReceiptView;

pub fn render_html(view: &ReceiptView) -> String {
    let rows = |items: &[(String, String)]| {
        items
            .iter()
            .map(|(left, right)| {
                format!(
                    r#"<div class="row"><span>{}</span><strong>{}</strong></div>"#,
                    escape(left),
                    escape(right)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{receipt_id} · Codex 小票</title>
  <style>
    :root {{
      color-scheme: light;
      --paper: #fff;
      --ink: #171717;
      --stage: #ececec;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      background: var(--stage);
      color: var(--ink);
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      display: grid;
      place-items: start center;
      padding: 16px;
    }}
    button {{
      margin: 0 0 12px;
      border: 0;
      border-radius: 999px;
      padding: 10px 16px;
      background: #181818;
      color: #fff;
      font: inherit;
      cursor: pointer;
    }}
    article {{
      width: min(80mm, calc(100vw - 24px));
      background: var(--paper);
      padding: 8mm 5mm 6mm;
    }}
    header, footer {{ text-align: center; }}
    .logo {{ font-size: 8mm; line-height: 1; font-weight: 900; }}
    .muted {{ margin-top: 2mm; font-size: 3.2mm; }}
    .rule {{ border-top: .35mm solid var(--ink); margin: 3mm 0; }}
    .strong {{ border-top-width: .55mm; }}
    .row {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 4mm;
      font-size: 3.35mm;
      line-height: 1.35;
    }}
    .row strong {{ text-align: right; white-space: nowrap; }}
    .footer {{ font-size: 3.5mm; line-height: 1.4; }}
    .barcode {{ margin-top: 3mm; white-space: pre; overflow: hidden; }}
    @page {{ size: 80mm auto; margin: 0; }}
    @media print {{
      body {{ background: #fff; padding: 0; display: block; }}
      button {{ display: none; }}
      article {{ width: 80mm; margin: 0 auto; }}
    }}
  </style>
</head>
<body>
  <button type="button" onclick="window.print()">打印小票</button>
  <article>
    <header>
      <div class="logo">█████</div>
      <div>CODEX</div>
      <div class="muted">感谢使用 Codex</div>
      <div class="muted">小票号: {receipt_id}</div>
      <div class="muted">日期: {date}</div>
    </header>
    <div class="rule strong"></div>
    {summary_rows}
    <div class="rule"></div>
    <div class="row"><span>项目</span><strong>TOKEN</strong></div>
    <div class="rule"></div>
    {token_rows}
    <div class="rule strong"></div>
    <div class="row"><span>{total_label}</span><strong>{total_value}</strong></div>
    <div class="rule"></div>
    {pricing_rows}
    <footer>
      <div class="rule strong"></div>
      <div class="footer">{footer}</div>
      <div class="barcode">{barcode}</div>
      <div class="muted">{receipt_id}</div>
    </footer>
  </article>
</body>
</html>"#,
        receipt_id = escape(&view.receipt_id),
        date = escape(&view.date),
        summary_rows = rows(&view.summary_rows),
        token_rows = rows(&view.token_rows),
        total_label = escape(&view.total_row.0),
        total_value = escape(&view.total_row.1),
        pricing_rows = rows(&view.pricing_rows),
        footer = escape(&view.footer),
        barcode = escape(&view.barcode),
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

- [ ] **Step 2: Wire HTML output**

In `src/cli.rs`, replace the HTML branch:

```rust
        OutputFormat::Html => crate::render_html::render_html(&view),
```

- [ ] **Step 3: Add HTML output test**

Append to `tests/cli_receipt_test.rs`:

```rust
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
```

- [ ] **Step 4: Run HTML test**

Run:

```bash
cargo test writes_printable_html_receipt
```

Expected: PASS.

## Task 6: Width Validation and Field Report

**Files:**

- Modify: `src/render_text.rs`
- Modify: `src/cli.rs`
- Modify: `tests/cli_receipt_test.rs`

- [ ] **Step 1: Add text width regression test**

Append to `tests/cli_receipt_test.rs`:

```rust
#[test]
fn receipt_lines_do_not_exceed_width() {
    let mut cmd = Command::cargo_bin("codex-receipt").unwrap();
    let output = cmd
        .args(["--session", "tests/fixtures/codex-session.jsonl", "--width", "42"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for line in text.lines() {
        let width = unicode_width::UnicodeWidthStr::width(line);
        assert!(width <= 42, "line too wide: {width} > 42: {line:?}");
    }
}
```

- [ ] **Step 2: Add unmapped model test**

Append:

```rust
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
```

- [ ] **Step 3: Run all CLI tests**

Run:

```bash
cargo test --test cli_receipt_test
```

Expected: PASS.

## Task 7: Documentation and Release Metadata

**Files:**

- Create: `README.md`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add README**

Add `README.md`:

```markdown
# codex-receipt

把 Codex 本地会话里的 token 用量，打印成一张中文小票。

不是 dashboard。不是趋势图。不是团队报表。

它只做一件事：让一次 AI 使用成本变成可以贴进聊天框、截图、转发的小票。

## Quick Start

```bash
cargo install --path .
codex-receipt
```

指定会话文件：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl
```

导出 HTML：

```bash
codex-receipt --output html --write receipt.html
```

查看日志里可证明的字段：

```bash
codex-receipt --show-fields
```

## v1 Scope

- 支持 Codex 本地 JSONL 会话。
- 默认读取最新 `.codex/sessions` 或 `.codex/archived_sessions`。
- 默认输出中文文本小票。
- 支持 HTML 打印页。
- 价格来自 `references/pricing.json`，只做估算，不等于真实账单。

## Not Included

- 不做 Claude Code。
- 不做 Trae。
- 不做历史 dashboard。
- 不做 leaderboard。
- 不联网更新价格。

## Validation

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```
```

- [ ] **Step 2: Add CI**

Add `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
```

- [ ] **Step 3: Add release workflow**

Add `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: codex-receipt-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/codex-receipt*
```

- [ ] **Step 4: Run final local checks**

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Expected: all pass.

## Acceptance Criteria

- `codex-receipt --session tests/fixtures/codex-session.jsonl` prints a Chinese text receipt.
- Receipt contains Codex branding, receipt id, date, provider, model, context used, token rows, total, price estimate, footer, barcode.
- Receipt lines stay within the selected width for Chinese output.
- `--scope session` uses `total_token_usage`.
- `--show-fields` prints JSON that includes available token fields.
- Unknown model renders a receipt and shows `价格未映射`.
- `--output html --write receipt.html` writes printable HTML and emits no stdout.
- `cargo test` passes.

## Explicit Non-Goals

- No multi-host support in v1.
- No automatic price update in v1.
- No global install scripts in v1.
- No dashboard in v1.
- No QR code in v1.
- No claim that estimated cost equals official billing.

## Follow-Up After v1

- Add GitHub Releases installation instructions.
- Add Homebrew and Scoop packages.
- Add optional Claude Code support only after Codex receipt quality is strong.
- Add `--footer-tone dry|snarky|encouraging`.
- Add project-level weekly receipt only if it preserves the artifact-first feel.
