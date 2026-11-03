//! NapCat OneBot v11 API 查询客户端：动作帧经动作通道发出，回执按 echo 关联。
//! 出站动作发送方共用一个 echo 计数器（next_echo），避免与查询冲突。

use anyhow::Context;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct ApiClient {
    action_tx: mpsc::Sender<Value>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    echo: Arc<AtomicU64>,
}

impl ApiClient {
    pub fn new(action_tx: mpsc::Sender<Value>) -> Self {
        Self {
            action_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            echo: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 分配全局唯一 echo（出站动作与查询共用）。
    pub fn next_echo(&self) -> u64 {
        self.echo.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 调用 OneBot API，等待回执并返回 data 字段。
    /// 调用方应自行包 tokio::time::timeout 控制等待时长（Python 版依赖 requests 超时）。
    pub async fn call(&self, action: &str, params: Value) -> anyhow::Result<Value> {
        let echo = self.next_echo();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(echo, tx);
        let frame = json!({ "action": action, "params": params, "echo": echo });
        self.action_tx
            .send(frame)
            .await
            .context("onebot action channel closed")?;
        let receipt = rx.await.context("no receipt for onebot api call")?;
        if receipt.get("status").and_then(Value::as_str) == Some("failed") {
            anyhow::bail!(
                "onebot api {action} failed: {}",
                receipt.get("wording").or(receipt.get("message")).cloned().unwrap_or(Value::Null)
            );
        }
        Ok(receipt.get("data").cloned().unwrap_or(Value::Null))
    }

    /// 处理一条 API 回执；匹配到等待中的查询则消费并返回 true。
    pub fn resolve_receipt(&self, receipt: &Value) -> bool {
        let Some(echo) = receipt.get("echo").and_then(Value::as_u64) else {
            return false;
        };
        match self.pending.lock().unwrap().remove(&echo) {
            Some(tx) => {
                let _ = tx.send(receipt.clone());
                true
            }
            None => false,
        }
    }
}
