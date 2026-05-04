//! 负责加载静态价格表、按模型名或别名匹配价格，并估算费用。
//! 本模块不读取 Codex 日志，也不决定小票展示字段。

use crate::models::{
    normalize_key, PriceEstimate, PriceStatus, PricingEntry, PricingTable, UsageSnapshot,
};
use anyhow::{Context, Result};
use std::path::Path;

/// 返回 v1 默认价格表路径。
pub fn default_pricing_path() -> &'static Path {
    Path::new("references/pricing.json")
}

/// 根据用量快照估算费用。
///
/// 找不到匹配价格时返回 `PriceStatus::Unmapped`，仍允许调用方继续渲染小票。
pub fn estimate_cost(snapshot: &UsageSnapshot, pricing_path: &Path) -> Result<PriceEstimate> {
    let table = load_pricing(pricing_path)?;
    let currency = table.currency.clone().unwrap_or_else(|| "USD".to_string());
    let unit = table.unit.as_deref().unwrap_or("per_1m_tokens");
    let Some(entry) = find_price(&table, &snapshot.model) else {
        return Ok(PriceEstimate {
            status: PriceStatus::Unmapped,
            amount: None,
            currency,
            model: "价格未映射".to_string(),
            source_checked_at: table.checked_at.clone(),
            rate_note: None,
        });
    };

    let cached = snapshot.cached_input_tokens.min(snapshot.input_tokens);
    let uncached = snapshot.input_tokens.saturating_sub(cached);
    let cached_rate = entry
        .cached_input_per_million
        .unwrap_or(entry.input_per_million);
    let amount = (uncached as f64 * entry.input_per_million
        + cached as f64 * cached_rate
        + snapshot.output_tokens as f64 * entry.output_per_million)
        / 1_000_000.0;

    Ok(PriceEstimate {
        status: PriceStatus::Estimate,
        amount: Some(amount),
        currency: entry.currency.clone().unwrap_or(currency),
        model: entry.model.clone(),
        source_checked_at: entry.source_checked_at.clone().or(table.checked_at.clone()),
        rate_note: price_note(entry, unit),
    })
}

fn load_pricing(path: &Path) -> Result<PricingTable> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取价格表 {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("无法解析价格表 {}", path.display()))
}

fn find_price<'a>(table: &'a PricingTable, model: &str) -> Option<&'a PricingEntry> {
    let model_key = normalize_key(model);

    table.models.iter().find(|entry| {
        normalize_key(&entry.model) == model_key
            || entry
                .aliases
                .iter()
                .any(|alias| normalize_key(alias) == model_key)
    })
}

fn price_note(entry: &PricingEntry, unit: &str) -> Option<String> {
    match (&entry.rate_note, &entry.source_url) {
        (Some(note), _) => Some(note.clone()),
        (None, Some(url)) => Some(format!("{unit}; {url}")),
        (None, None) => Some(unit.to_string()),
    }
}
