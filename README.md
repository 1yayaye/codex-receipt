# codex-receipt

把 Codex 本地会话里的 token 用量打印成一张中文小票。

它不是 dashboard，不是趋势图，也不是团队报表。v1 只做一件事：把一次 Codex 使用记录变成适合聊天、截图和转发的文本或 HTML 小票。

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

## 运行用例

从本机最新 Codex 会话生成默认文本小票：

```bash
codex-receipt
```

使用测试 fixture 生成可复现的小票：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl
```

统计整场会话，而不是最近一轮：

```bash
codex-receipt --session tests/fixtures/codex-session.jsonl --scope session
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

## Options

```text
--session <PATH>              使用指定 Codex JSONL 会话文件
--scope <latest-turn|session> 读取最近一轮或整场会话，默认 latest-turn
--width <42|48|56|64>         文本小票宽度，默认 48
--output <text|html>          输出文本或可打印 HTML，默认 text
--write <PATH>                写入文件并抑制 stdout
--pricing <PATH>              使用自定义价格表
--model <MODEL>               覆盖模型展示和价格匹配
--provider <PROVIDER>         覆盖供应商展示和价格匹配
--show-fields                 输出字段可用性 JSON
```

## v1 Scope

- 只支持 Codex 本地 JSONL 会话。
- 默认读取最新的 `.codex/sessions` 或 `.codex/archived_sessions` 文件。
- 默认输出中文文本小票。
- 支持 HTML 打印页。
- 价格来自 `references/pricing.json`，只做估算，不等于真实账单。

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
