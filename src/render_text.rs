//! 负责固定宽度中文文本小票渲染。
//! 本模块不决定业务字段，也不估算价格。

use crate::receipt::ReceiptView;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// 把小票视图模型渲染为文本。
pub fn render_text(view: &ReceiptView) -> String {
    let mut out = Vec::new();
    center(&mut out, "█████", view.width);
    center(&mut out, &view.title, view.width);
    out.push(String::new());
    center(&mut out, "感谢使用 Codex", view.width);
    center(&mut out, &format!("小票号 {}", view.receipt_id), view.width);
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
    let right = truncate(right, width.saturating_sub(2));
    let right_width = UnicodeWidthStr::width(right.as_str());
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
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
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
