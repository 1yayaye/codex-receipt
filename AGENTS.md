# AGENTS.md

This file defines how agents should work in this repository.

It is a project-level behavior guide. It does not replace the implementation plan. For the current v1, the source of truth is:

`docs/superpowers/plans/2026-05-04-codex-receipt.md`

If the plan is wrong, incomplete, or conflicts with the code that exists later, do not silently work around it. State the mismatch, explain the reason, and update the plan or ask for confirmation before changing implementation direction.

## Project Goal

This repository is for a Rust CLI that reads local Codex session logs and prints Chinese, screenshot-ready token usage receipts.

The v1 product direction is intentionally narrow:

- Codex first.
- Chinese receipt feel first.
- Chat output first.
- Rust binary distribution first.

Do not turn this project into a general AI usage dashboard, leaderboard, telemetry service, or multi-agent analytics platform unless a new approved plan explicitly changes the scope.

## Working Standard

Use medium-strength discipline.

Small documentation, typo, comment, and formatting fixes can be handled directly. Behavior changes need analysis first.

For behavior bugs, parser bugs, rendering alignment bugs, pricing mistakes, CLI behavior changes, or any change that affects user-visible output:

1. Identify the root cause.
2. Identify the module that owns the behavior.
3. Add or update a focused test.
4. Make the smallest correct change in the owning module.
5. Run relevant verification.

Do not patch around symptoms.

Bad patterns:

- Adding a condition at the call site when the invariant belongs in a parser or renderer.
- Hard-coding one fixture or one model name to make a test pass.
- Copying logic between modules instead of moving it to the owner.
- Bypassing tests because the visual output "looks fine".
- Mixing parsing, pricing, and rendering logic in one file for convenience.

Good patterns:

- Fix Codex JSONL interpretation in `src/codex_logs.rs`.
- Fix price matching in `src/pricing.rs`.
- Fix receipt row selection in `src/receipt.rs`.
- Fix text alignment in `src/render_text.rs`.
- Fix HTML-only presentation in `src/render_html.rs`.
- Keep `src/cli.rs` as orchestration, not a business logic dump.

## Plan-First Workflow

Before implementation work:

1. Read `docs/superpowers/plans/2026-05-04-codex-receipt.md`.
2. Read the files related to the task.
3. Check existing tests or fixtures that describe the behavior.
4. State what kind of change this is: feature, bugfix, refactor, docs, or test.

During implementation:

1. Follow the current implementation plan task-by-task.
2. If a step is technically wrong, stop and record the reason.
3. Prefer updating the plan before changing direction.
4. Keep changes scoped to the task.
5. Do not introduce unrelated refactors.

After implementation:

1. Report what changed.
2. Report which verification commands ran.
3. Report remaining risks or skipped checks.

## Test-First Rule

For features and bugfixes, write or update a failing test before changing production code.

Exceptions:

- Documentation-only changes.
- Comments.
- Formatting-only changes.
- Mechanical metadata updates that do not affect behavior.

Tests should cover user-visible behavior, not just internal function calls. For this CLI, prefer fixture-driven tests using stable files under `tests/fixtures/`.

Never depend on the developer's real local Codex logs in automated tests.

## Project Structure Rules

Keep module boundaries clear.

`src/cli.rs`

- Owns CLI argument parsing.
- Owns top-level flow orchestration.
- Owns stdout vs `--write` behavior.
- Must not parse Codex JSONL details.
- Must not calculate prices.
- Must not hand-build receipt lines.

`src/codex_logs.rs`

- Owns Codex session discovery.
- Owns JSONL parsing.
- Owns extraction of `session_meta`, `turn_context`, and `token_count`.
- Must not format receipt rows.
- Must not estimate prices.

`src/pricing.rs`

- Owns pricing file loading.
- Owns provider/model/alias matching.
- Owns cost estimation.
- Owns unmapped-price fallback.
- Must preserve pricing source metadata.

`src/receipt.rs`

- Owns conversion from `UsageSnapshot` plus `PriceEstimate` into a receipt view model.
- Owns which fields appear on the receipt.
- Must not parse files.
- Must not contain terminal-width rendering code.

`src/render_text.rs`

- Owns fixed-width text rendering.
- Owns Chinese display-width handling.
- Owns text truncation and alignment.
- Must not decide business fields.
- Must not estimate prices.

`src/render_html.rs`

- Owns printable HTML receipt output.
- Must not alter receipt business data.
- Must not duplicate pricing or parser rules.

`tests/fixtures/`

- Contains stable Codex JSONL examples.
- Must not contain machine-local absolute paths.
- Must be deterministic.

`references/pricing.json`

- Is the static v1 pricing table.
- Any price update must include source URL and checked date.
- Do not claim price estimates are official billing records.

## Modification Workflow

Use this workflow for non-trivial changes:

1. Read the plan, relevant modules, and tests.
2. Classify the change in one sentence.
3. Locate the root cause and owning module.
4. Write or update the focused test first.
5. Run the test and confirm it fails for the expected reason.
6. Implement the minimal correct change.
7. Run the focused test again.
8. Run broader verification before claiming completion.
9. Summarize changes and risks.

For trivial docs/comment/typo changes, use judgment and keep the edit small.

## Verification

Before claiming implementation work is complete, run fresh verification.

Default full verification:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For local iteration, it is fine to run smaller commands first, such as:

```bash
cargo test renders_chinese_receipt_from_fixture
cargo test --test cli_receipt_test
cargo check
```

If a command cannot be run because the project is not scaffolded yet, say that directly. Do not imply tests passed.

Rendering changes must verify:

- Chinese receipt lines do not exceed the selected width.
- Unknown models render `价格未映射`.
- `--write` suppresses stdout.
- HTML output contains printable page structure.

Parser changes must verify:

- `latest-turn` reads `last_token_usage`.
- `session` reads `total_token_usage`.
- Missing `token_count` returns a clear error.
- Tests use fixtures, not real local logs.

Pricing changes must verify:

- Alias matching works.
- Unmapped models still render a receipt.
- Currency label and amount formatting are correct.
- Source checked date is preserved.

## Chinese Text Writing Rules

Do not embed Chinese body text directly in command lines, PowerShell here-strings, or `@'...'@ | python -` style pipelines.

If `apply_patch` is available and stable, use `apply_patch` for file writes.

If `apply_patch` is unavailable or unstable, write through a local UTF-8 script file and run that file. Do not put Chinese prose in the command line itself.

## Rust Style

- Keep files focused.
- Prefer explicit structs over loose JSON plumbing after parsing.
- Keep fallbacks honest. Unknown means unknown.
- Avoid global mutable state.
- Avoid network access in core behavior.
- Keep CLI errors actionable.
- Do not add dependencies without a clear reason.

## Chinese Comments

Use Chinese comments for modules and functions.

Required comments:

- Each Rust module should start with a Chinese module-level comment explaining what the module owns and what it must not do.
- Each public function should have a Chinese doc comment explaining its purpose, inputs, output, and important failure behavior.
- Non-trivial private functions should have a short Chinese comment when their responsibility, invariant, or edge case is not obvious from the name.

Comment quality rules:

- Comments should explain intent, boundaries, invariants, and business rules.
- Do not write line-by-line Chinese translations of obvious code.
- Do not use comments to excuse confusing code. If the code needs a long defensive explanation, prefer simplifying the code or splitting the function.
- Keep comments synchronized with behavior. Updating code without updating stale comments is a bug.

Examples:

```rust
//! 负责读取 Codex 本地会话日志，并把 JSONL 事件转换成稳定的用量快照。
//! 本模块不负责价格估算，也不负责小票排版。

/// 从指定 Codex JSONL 文件读取用量快照。
///
/// `scope` 决定读取最近一轮还是整场会话。缺少 `token_count` 事件时返回错误，
/// 调用方应把这个错误展示成可操作的 CLI 提示。
pub fn load_snapshot_from_session(...) -> Result<UsageSnapshot> {
    ...
}
```

## Product Guardrails

This project wins by producing a good artifact, not by showing more metrics.

Preserve these defaults unless an approved plan changes them:

- Main output is a Chinese text receipt.
- HTML is secondary.
- Codex is the only v1 automatic data source.
- Pricing is estimated from a static table.
- Missing data is omitted or clearly marked.
- The receipt should be screenshot-worthy.

If a proposed change makes the product feel more like a dashboard, stop and justify why it belongs in this repo.
