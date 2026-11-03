//! 出站发送器：统一 OneBot 动作发送 + echo 回执关联。
//!
//! - 查询类回执由 ApiClient 消费（见 onebot/api.rs）
//! - 出站动作回执由本结构消费：send_and_wait 注册等待方，分派协程收到回执时 resolve
//! 对齐 SnowLuma-Adapter：出站 invoke 是串行 await 发送回执的，echo 回调在同一次响应里带回。

use anyhow::Context;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::outbound::OutboundAction;

pub const SEND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

pub struct OutboundSender {
    action_tx: mpsc::Sender<Value>,
    echo: Arc<crate::onebot::api::ApiClient>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
}

impl OutboundSender {
    pub fn new(action_tx: mpsc::Sender<Value>, echo: Arc<crate::onebot::api::ApiClient>) -> Self {
        Self {
            action_tx,
            echo,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 发送动作；wait=true 时等待平台回执并返回回执帧（超时返回 None）。
    pub async fn send(&self, action: OutboundAction, wait: bool) -> anyhow::Result<Option<Value>> {
        let echo_id = self.echo.next_echo();
        let frame = json!({ "action": action.action, "params": action.params, "echo": echo_id });
        let rx = if wait {
            let (tx, rx) = oneshot::channel();
            self.pending.lock().unwrap().insert(echo_id, tx);
            Some(rx)
        } else {
            None
        };
        if let Err(e) = self.action_tx.send(frame).await {
            if let Some(rx) = rx {
                drop(rx);
                self.pending.lock().unwrap().remove(&echo_id);
            }
            return Err(e).context("onebot action channel closed");
        }
        let Some(rx) = rx else { return Ok(None) };
        match tokio::time::timeout(SEND_TIMEOUT, rx).await {
            Ok(Ok(receipt)) => Ok(Some(receipt)),
            Ok(Err(_)) => anyhow::bail!("connection closed while waiting send receipt"),
            Err(_) => {
                self.pending.lock().unwrap().remove(&echo_id);
                warn!(echo = echo_id, "send receipt timeout");
                Ok(None)
            }
        }
    }

    /// 分派协程收到回执时调用；匹配到等待方返回 true。
    pub fn resolve_receipt(&self, receipt: &Value) -> bool {
        let Some(echo_id) = receipt.get("echo").and_then(Value::as_u64) else {
            return false;
        };
        match self.pending.lock().unwrap().remove(&echo_id) {
            Some(tx) => {
                let _ = tx.send(receipt.clone());
                true
            }
            None => false,
        }
    }
}
