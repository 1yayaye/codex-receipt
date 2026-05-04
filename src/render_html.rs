//! 负责生成可打印的 HTML 小票。
//! 本模块不改变小票业务数据，也不复制价格或解析规则。

use crate::receipt::ReceiptView;

/// 把小票视图模型渲染为可打印 HTML。
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
  <title>{receipt_id} - Codex 小票</title>
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
      <div>{title}</div>
      <div class="muted">感谢使用 Codex</div>
      <div class="muted">小票号 {receipt_id}</div>
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
        title = escape(&view.title),
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
