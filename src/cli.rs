//! 负责命令行参数解析和顶层流程编排。
//! 本模块不解析 Codex JSONL 细节，不计算价格，也不手写小票行。

use crate::desktop_notify::SystemNotifier;
use crate::models::{OutputFormat, Scope};
use crate::opener::SystemOpener;
use crate::watch::WatchConfig;
use anyhow::{Context, Result};
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "codex-receipt")]
#[command(about = "Print Codex token usage as a Chinese receipt")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub receipt: ReceiptArgs,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Watch(WatchArgs),
    NotifyTest(NotifyTestArgs),
}

#[derive(Debug, ClapArgs)]
pub struct ReceiptArgs {
    #[arg(long)]
    pub session: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = ScopeArg::Session)]
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

#[derive(Debug, ClapArgs)]
pub struct WatchArgs {
    #[arg(long)]
    pub receipt_dir: Option<PathBuf>,

    #[arg(long)]
    pub state: Option<PathBuf>,

    #[arg(long)]
    pub replay_existing: bool,

    #[arg(long)]
    pub once: bool,

    #[arg(long)]
    pub no_open: bool,
}

#[derive(Debug, ClapArgs)]
pub struct NotifyTestArgs {
    #[arg(long)]
    pub receipt_dir: Option<PathBuf>,

    #[arg(long)]
    pub no_open: bool,
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
    match args.command {
        Some(Command::Watch(watch_args)) => run_watch(watch_args),
        Some(Command::NotifyTest(test_args)) => run_notify_test(test_args),
        None => run_receipt(args.receipt),
    }
}

fn run_watch(args: WatchArgs) -> Result<()> {
    let config = WatchConfig {
        archived_dir: crate::paths::default_archived_sessions_dir()?,
        receipt_dir: match args.receipt_dir {
            Some(path) => path,
            None => crate::paths::default_receipt_dir()?,
        },
        state_path: match args.state {
            Some(path) => path,
            None => crate::paths::default_state_path()?,
        },
        pricing_path: crate::pricing::default_pricing_path().to_path_buf(),
        replay_existing: args.replay_existing,
        once: args.once,
        open_receipt: !args.no_open,
    };
    let notifier = SystemNotifier;
    let opener = SystemOpener;
    crate::watch::run_watch(config, &notifier, &opener)
}

fn run_notify_test(args: NotifyTestArgs) -> Result<()> {
    let receipt_dir = match args.receipt_dir {
        Some(path) => path,
        None => crate::paths::default_receipt_dir()?,
    };
    let notifier = SystemNotifier;
    let opener = SystemOpener;
    let report = crate::watch::run_notify_test(&receipt_dir, !args.no_open, &notifier, &opener)
        .with_context(|| "通知测试失败")?;

    println!("测试小票: {}", report.receipt_path.display());
    println!("系统通知: {}", report.notification_status);
    println!("打开小票: {}", report.open_status);
    Ok(())
}

fn run_receipt(args: ReceiptArgs) -> Result<()> {
    let session_path = match args.session.as_ref() {
        Some(path) => path.clone(),
        None => crate::codex_logs::newest_session_file()?,
    };
    let generated = generate_receipt_from_session(
        &session_path,
        GenerateOptions {
            scope: args.scope.into(),
            model: args.model.as_deref(),
            provider: args.provider.as_deref(),
            pricing: args.pricing.as_deref(),
            output: args.output.into(),
            width: args.width,
            show_fields: args.show_fields,
        },
    )?;

    write_or_print(args.write.as_deref(), generated.content)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct GeneratedReceipt {
    pub content: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerateOptions<'a> {
    pub scope: Scope,
    pub model: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub pricing: Option<&'a Path>,
    pub output: OutputFormat,
    pub width: usize,
    pub show_fields: bool,
}

/// 从指定会话生成一次性输出内容。
///
/// 该函数复用解析、定价和渲染链路，供普通 CLI 输出和后台 watch 模式共享行为边界。
pub fn generate_receipt_from_session(
    session_path: &Path,
    options: GenerateOptions<'_>,
) -> Result<GeneratedReceipt> {
    let snapshot = crate::codex_logs::load_snapshot_from_session(
        session_path,
        options.scope,
        options.model,
        options.provider,
    )?;

    if options.show_fields {
        let json = serde_json::to_string_pretty(&snapshot)?;
        return Ok(GeneratedReceipt { content: json });
    }

    let pricing_path = match options.pricing {
        Some(path) => path,
        None => crate::pricing::default_pricing_path(),
    };
    let estimate = crate::pricing::estimate_cost(&snapshot, pricing_path)?;
    let view = crate::receipt::build_receipt_view(&snapshot, &estimate, options.width);
    let content = match options.output {
        OutputFormat::Text => crate::render_text::render_text(&view),
        OutputFormat::Html => crate::render_html::render_html(&view),
    };

    Ok(GeneratedReceipt { content })
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
