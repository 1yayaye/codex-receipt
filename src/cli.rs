//! 负责命令行参数解析和顶层流程编排。
//! 本模块不解析 Codex JSONL 细节，不计算价格，也不手写小票行。

use crate::models::{OutputFormat, Scope};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "codex-receipt")]
#[command(about = "Print Codex token usage as a Chinese receipt")]
pub struct Args {
    #[arg(long)]
    pub session: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = ScopeArg::LatestTurn)]
    pub scope: ScopeArg,

    #[arg(long, default_value_t = 48, value_parser = parse_width)]
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

/// 执行 CLI 主流程。
///
/// 根据参数选择会话文件、读取用量、估算价格，并把结果输出到 stdout 或 `--write`
/// 指定的文件。底层读取或渲染失败时返回可展示的错误。
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
        write_or_print(args.write.as_deref(), json)?;
        return Ok(());
    }

    let pricing_path = match args.pricing.as_deref() {
        Some(path) => path,
        None => crate::pricing::default_pricing_path(),
    };
    let estimate = crate::pricing::estimate_cost(&snapshot, pricing_path)?;
    let view = crate::receipt::build_receipt_view(&snapshot, &estimate, args.width);
    let rendered = match OutputFormat::from(args.output) {
        OutputFormat::Text => crate::render_text::render_text(&view),
        OutputFormat::Html => crate::render_html::render_html(&view),
    };

    write_or_print(args.write.as_deref(), rendered)?;
    Ok(())
}

fn write_or_print(path: Option<&Path>, content: String) -> Result<()> {
    if let Some(path) = path {
        std::fs::write(path, format!("{content}\n"))?;
    } else {
        println!("{content}");
    }
    Ok(())
}

fn parse_width(value: &str) -> std::result::Result<usize, String> {
    let width = value
        .parse::<usize>()
        .map_err(|_| "width must be one of 42, 48, 56, 64".to_string())?;
    match width {
        42 | 48 | 56 | 64 => Ok(width),
        _ => Err("width must be one of 42, 48, 56, 64".to_string()),
    }
}
