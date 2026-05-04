# codex-receipt

把 Codex 本地会话里的 token 用量打印成一张中文小票。

它不是 dashboard，不是趋势图，也不是团队报表。v1 只做一件事：把一次 Codex 使用记录变成适合聊天、截图和转发的文本或 HTML 小票。

## Quick Start

```bash
cargo install --path .
codex-receipt
```

## Install From GitHub Releases

v2 publishes downloadable release assets from GitHub Releases when a `v*.*.*` tag is pushed.
Choose the archive for your platform:

```text
codex-receipt-x86_64-unknown-linux-gnu.tar.gz
codex-receipt-x86_64-apple-darwin.tar.gz
codex-receipt-aarch64-apple-darwin.tar.gz
codex-receipt-x86_64-pc-windows-msvc.zip
SHA256SUMS.txt
```

Verify the downloaded archive before installing:

```bash
sha256sum -c SHA256SUMS.txt
```

On Windows, use `Get-FileHash` and compare the SHA256 value with `SHA256SUMS.txt`.
After extraction, put `codex-receipt` or `codex-receipt.exe` somewhere on your `PATH`.
See `RELEASE.md` for the release checklist.

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

## 运行用例

从本机最新 Codex 会话生成默认文本小票：

```bash
codex-receipt
```

使用测试 fixture 生成可复现的小票：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl
```

只统计最近一轮，而不是整场会话：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --scope latest-turn
```

生成更窄的截图宽度：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --width 42
```

导出可打印 HTML，并且不向 stdout 输出内容：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --output html --write receipt.html
```

查看解析到的原始可用字段，适合排查 Codex 日志结构变化：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --show-fields
```

覆盖模型名，验证未知模型仍然能生成小票并显示 `价格未映射`：

```bash
codex-receipt --session tests/fixtures/codex-session-missing-model.jsonl --model unknown-model
```

使用自定义价格表：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --pricing references/pricing.json
```

后台监听 Codex 归档会话，完成后生成 HTML 小票并弹出系统通知：

```bash
codex-receipt watch
```

默认还会自动打开生成的 HTML 小票，避免系统通知被静默吞掉时完全没有可见反馈。只想保留通知和文件输出时：

```bash
codex-receipt watch --no-open
```

指定后台小票目录和状态文件：

```bash
codex-receipt watch --receipt-dir receipts --state watch-state.json
```

默认启动时不会补弹历史归档；需要处理状态文件里没见过的历史归档时使用：

```bash
codex-receipt watch --replay-existing
```

脚本或测试中只处理第一个新归档后退出：

```bash
codex-receipt watch --once
```

测试当前系统是否允许弹出和打开小票：

```bash
codex-receipt notify-test
```

只测试系统通知和文件生成，不打开 HTML：

```bash
codex-receipt notify-test --no-open
```

如果没有弹出，先看命令输出里的 `系统通知:` 和 `打开小票:`。Windows 专注助手、通知权限、后台脚本权限或未注册的通知来源都可能让系统通知失败；这种情况下 HTML 小票仍会生成，默认自动打开会提供更可靠的可见反馈。

## Options

```text
--session <PATH>              使用指定 Codex JSONL 会话文件
--scope <latest-turn|session> 读取最近一轮或整场会话，默认 session
--width <42|48|56|64>         文本小票宽度，默认 48
--output <text|html>          输出文本或可打印 HTML，默认 text
--write <PATH>                写入文件并抑制 stdout
--pricing <PATH>              使用自定义价格表
--model <MODEL>               覆盖模型展示和价格匹配
--provider <PROVIDER>         覆盖供应商展示
--show-fields                 输出字段可用性 JSON
```

后台子命令：

```text
codex-receipt watch
  --receipt-dir <PATH>        HTML 小票输出目录
  --state <PATH>              已通知会话状态文件
  --replay-existing           处理未记录的历史归档
  --no-open                   不自动打开生成的 HTML 小票
  --once                      处理第一个新归档后退出

codex-receipt notify-test
  --receipt-dir <PATH>        测试 HTML 小票输出目录
  --no-open                   不自动打开测试小票
```

## v1 Scope

- 只支持 Codex 本地 JSONL 会话。
- 默认读取最新的 `.codex/sessions` 或 `.codex/archived_sessions` 文件。
- 默认按整场会话累计口径生成小票，对齐 Codex CLI 退出时的用量摘要。
- 默认输出中文文本小票。
- 支持 HTML 打印页。
- 支持 `watch` 后台监听 Codex 归档会话，完成后写 HTML 小票并弹出系统通知。
- 价格来自 `references/pricing.json`，按模型名匹配，不受日志供应商字段影响。
- 默认价格按 OpenAI API 标准价估算，不等于 Codex、ChatGPT 或 OpenAI 官方真实账单。

## Not Included

- 不做 Claude Code 支持。
- 不做 Trae 支持。
- 不做历史 dashboard。
- 不做 leaderboard。
- 不联网更新价格。

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
