//! 存放跨模块共享的数据结构和轻量格式化工具。
//! 本模块不负责读取文件、估算价格或渲染最终小票。

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
    /// 最近一次请求占用的输入上下文 token，用于展示上下文窗口占用。
    pub context_input_tokens: u64,
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
    #[serde(rename = "provider")]
    pub _provider: String,
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

/// 把整数格式化成带英文千分位的稳定展示文本。
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

/// 生成用于模型名和别名匹配的宽松键。
///
/// 只保留 ASCII 字母数字并转小写，让 `gpt-5.4` 和 `GPT 5.4` 能匹配到同一项。
pub fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}
