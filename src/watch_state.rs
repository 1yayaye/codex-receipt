//! 负责后台 watch 模式的跨重启去重状态。
//! 本模块只维护已处理会话记录，不解析 Codex 日志，也不生成小票。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProcessedSession {
    pub key: String,
    pub session_id: String,
    pub source: PathBuf,
    pub token_timestamp: Option<String>,
    pub modified_ms: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WatchState {
    #[serde(default)]
    processed: BTreeSet<ProcessedSession>,
}

impl WatchState {
    /// 从指定路径读取状态文件。
    ///
    /// 文件不存在时返回空状态；文件存在但无法解析时返回错误，避免误重复通知。
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("无法读取 watch 状态文件 {}", path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("无法解析 watch 状态文件 {}", path.display()))
    }

    /// 判断记录是否已经处理过。
    pub fn contains(&self, record: &ProcessedSession) -> bool {
        self.processed.contains(record)
    }

    /// 插入一条已处理记录。
    pub fn insert(&mut self, record: ProcessedSession) {
        self.processed.insert(record);
    }

    /// 原子写入状态文件。
    ///
    /// 先写同目录临时文件再重命名，避免进程中断时留下半截 JSON。
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("无法创建 watch 状态目录 {}", parent.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self)?;
        fs::write(&tmp, format!("{text}\n"))
            .with_context(|| format!("无法写入临时状态文件 {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("无法更新 watch 状态文件 {}", path.display()))?;
        Ok(())
    }
}
