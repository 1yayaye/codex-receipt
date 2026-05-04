//! 负责把用量快照和价格估算转换为小票视图模型。
//! 本模块不读取文件、不计算价格，也不处理终端显示宽度。

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

/// 构建小票视图模型。
///
/// `date` 表示小票生成时间；日志里的 `token_count` 时间作为“统计时间”摘要行保留。
/// 根据日志里实际出现的 token 字段决定展示行，并保留未知价格的诚实回退。
pub fn build_receipt_view(
    snapshot: &UsageSnapshot,
    estimate: &PriceEstimate,
    width: usize,
) -> ReceiptView {
    let receipt_id = receipt_id(snapshot);
    let mut token_rows = Vec::new();

    if has_field(snapshot, "input_tokens") {
        token_rows.push(("输入 Tokens".to_string(), fmt_int(snapshot.input_tokens)));
    }
    if has_field(snapshot, "output_tokens") {
        token_rows.push(("输出 Tokens".to_string(), fmt_int(snapshot.output_tokens)));
    }
    if has_field(snapshot, "cached_input_tokens") {
        token_rows.push((
            "缓存读取".to_string(),
            fmt_int(snapshot.cached_input_tokens),
        ));
    }
    if has_field(snapshot, "reasoning_output_tokens") {
        token_rows.push((
            "推理 Tokens".to_string(),
            fmt_int(snapshot.reasoning_output_tokens),
        ));
    }

    ReceiptView {
        width,
        title: "CODEX 小票".to_string(),
        receipt_id: receipt_id.clone(),
        date: generated_time(),
        summary_rows: summary_rows(snapshot),
        token_rows,
        total_row: (
            "总计".to_string(),
            format!("{} Tokens", fmt_int(snapshot.total_tokens)),
        ),
        pricing_rows: pricing_rows(estimate),
        footer: choose_footer(snapshot),
        barcode: barcode(&receipt_id, width),
    }
}

fn has_field(snapshot: &UsageSnapshot, field: &str) -> bool {
    snapshot.available_fields.iter().any(|item| item == field)
}

fn summary_rows(snapshot: &UsageSnapshot) -> Vec<(String, String)> {
    let mut rows = vec![
        ("供应商".to_string(), snapshot.provider.to_uppercase()),
        ("模型".to_string(), snapshot.model.clone()),
        ("已用上下文".to_string(), context_used(snapshot)),
    ];

    if let Some(time) = display_time(snapshot.timestamp.as_deref()) {
        rows.push(("统计时间".to_string(), time));
    }

    rows
}

fn receipt_id(snapshot: &UsageSnapshot) -> String {
    let seed = format!(
        "{}:{}:{}:{}",
        snapshot.session_id, snapshot.provider, snapshot.model, snapshot.total_tokens
    );
    let digest = stable_digest(&seed);
    format!("CX_{}_{}", Local::now().format("%Y%m%d_%H%M%S"), digest)
}

fn stable_digest(seed: &str) -> String {
    let mut hash: u32 = 2_166_136_261;
    for byte in seed.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    format!("{:06X}", hash & 0xFF_FFFF)
}

fn generated_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn display_time(value: Option<&str>) -> Option<String> {
    value
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
}

fn context_used(snapshot: &UsageSnapshot) -> String {
    match snapshot.context_window {
        Some(window) => format!(
            "{}/{}",
            fmt_int(snapshot.context_input_tokens),
            fmt_int(window)
        ),
        None => fmt_int(snapshot.context_input_tokens),
    }
}

fn pricing_rows(estimate: &PriceEstimate) -> Vec<(String, String)> {
    let amount = match estimate.amount {
        Some(value) if estimate.currency == "USD" => format!("${value:.6}"),
        Some(value) if estimate.currency == "CNY" => format!("¥{value:.6}"),
        Some(value) => format!("{} {value:.6}", estimate.currency),
        None => "价格未映射".to_string(),
    };

    let mut rows = vec![(format!("{} 预估", estimate.currency), amount)];
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
        "推理不免费，账单更诚实。".to_string()
    } else if snapshot.cached_input_tokens > 0 {
        "缓存省了一点，仍然值得记录。".to_string()
    } else {
        "结果很体面，账单很诚实。".to_string()
    }
}

fn barcode(seed: &str, width: usize) -> String {
    let digest = stable_digest(seed);
    let target = width.saturating_sub(8).max(24);
    let mut bars = String::new();
    for ch in digest.chars().cycle() {
        let piece = match ch {
            '0'..='3' => "|",
            '4'..='7' => "||",
            '8'..='B' => "| ",
            _ => " ||",
        };
        if bars.len() + piece.len() > target {
            break;
        }
        bars.push_str(piece);
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UsageSnapshot;
    use std::path::PathBuf;

    #[test]
    fn receipt_date_is_generated_time_and_stat_time_is_separate() {
        let snapshot = UsageSnapshot {
            input_tokens: 100,
            context_input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 0,
            total_tokens: 120,
            context_window: None,
            provider: "openai".to_string(),
            model: "gpt-5.4".to_string(),
            source: PathBuf::from("tests/fixtures/codex-session.jsonl"),
            session_id: "fixture-session".to_string(),
            timestamp: Some("2026-05-04T01:00:02Z".to_string()),
            scope: "latest-turn".to_string(),
            available_fields: vec![
                "input_tokens".to_string(),
                "output_tokens".to_string(),
                "total_tokens".to_string(),
            ],
        };
        let estimate = PriceEstimate {
            status: PriceStatus::Unmapped,
            amount: None,
            currency: "USD".to_string(),
            model: "价格未映射".to_string(),
            source_checked_at: Some("2026-05-04".to_string()),
            rate_note: None,
        };

        let view = build_receipt_view(&snapshot, &estimate, 48);

        assert_ne!(view.date, "2026-05-04 09:00:02");
        assert!(view
            .summary_rows
            .iter()
            .any(|(label, value)| label == "统计时间" && value == "2026-05-04 09:00:02"));
    }
}
