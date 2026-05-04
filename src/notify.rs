//! 负责发送桌面通知，并为测试提供可替换的通知接口。
//! 本模块不生成小票，也不决定会话是否已经处理。

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationMessage {
    pub title: String,
    pub body: String,
}

/// 发送后台小票通知的抽象接口。
///
/// 生产环境使用系统通知；测试环境可以传入记录型实现，避免真实弹窗。
pub trait Notifier {
    fn notify(&self, message: &NotificationMessage) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemNotifier;

impl Notifier for SystemNotifier {
    fn notify(&self, message: &NotificationMessage) -> Result<()> {
        notify_rust::Notification::new()
            .summary(&message.title)
            .body(&message.body)
            .appname("codex-receipt")
            .show()?;
        Ok(())
    }
}
