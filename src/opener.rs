//! 负责打开生成的小票文件，并为测试提供可替换的打开接口。
//! 本模块不生成小票，也不决定何时触发弹出。

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

/// 打开小票文件的抽象接口。
///
/// 生产环境使用系统默认程序打开 HTML；测试环境可以传入记录型实现，避免真的打开窗口。
pub trait Opener {
    fn open(&self, path: &Path) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemOpener;

impl Opener for SystemOpener {
    fn open(&self, path: &Path) -> Result<()> {
        open_path(path)
    }
}

fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .status()?;
        if status.success() {
            return Ok(());
        }
        Err(anyhow!("打开小票失败: {}", path.display()))
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open").arg(path).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("打开小票失败: {}", path.display()));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = Command::new("xdg-open").arg(path).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("打开小票失败: {}", path.display()));
    }
}
