mod cli;
mod codex_logs;
mod models;
mod pricing;
mod receipt;
mod render_html;
mod render_text;

fn main() -> anyhow::Result<()> {
    cli::run()
}
