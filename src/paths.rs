//! 负责解析 Codex 日志目录、小票输出目录和后台状态文件路径。
//! 本模块不读取会话内容，也不触发通知。

use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// 返回当前用户的 Codex 归档会话目录。
///
/// 优先使用 `HOME`，在 Windows 上回退到 `USERPROFILE`。无法定位用户主目录时返回错误。
pub fn default_archived_sessions_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("无法定位用户主目录"))?;
    Ok(home.join(".codex").join("archived_sessions"))
}

/// 返回后台模式默认写入 HTML 小票的目录。
///
/// 使用系统数据目录下的 `codex-receipt/receipts`，避免把后台产物写进项目目录。
pub fn default_receipt_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.data_local_dir().join("receipts"))
}

/// 返回后台模式默认状态文件路径。
///
/// 状态文件记录已经通知过的归档会话，用于跨进程重启去重。
pub fn default_state_path() -> Result<PathBuf> {
    Ok(project_dirs()?.data_local_dir().join("watch-state.json"))
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "codex-receipt")
        .ok_or_else(|| anyhow!("无法定位 codex-receipt 用户数据目录"))
}
