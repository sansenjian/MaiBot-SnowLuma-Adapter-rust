//! 对 MaiBot Core 的 socket.io 上行客户端（maim_message MessageServer，ws 模式）。
//!
//! 协议要点（对齐 maim_message ws_connection.py，详见 docs/event-schema.md §5）：
//! - 握手 HTTP 头：`platform: <platform_name>`（Core 按此路由）、`Authorization: <token>`（可选）
//! - 单事件名 `"message"`，载荷为 MessageBase JSON 原样
//! - 断线重连：1s 起步指数退避至 60s
//!
//! 注意：rust-socketio 的 API 在不同版本间有差异（connect 是否异步、Payload 变体），
//! 首次编译若报错按所装版本微调本文件即可。

use rust_socketio::{ClientBuilder, Payload};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::config::Config;

/// - `emit_rx`：入站消息（MessageBase JSON），经 `"message"` 事件发往 Core
/// - `recv_tx`：Core 下发的消息（含自定义信封），转交出站管线
pub async fn run(
    config: Config,
    mut emit_rx: mpsc::Receiver<Value>,
    recv_tx: mpsc::Sender<Value>,
) -> anyhow::Result<()> {
    let mut backoff = config.reconnect_min_interval;
    loop {
        info!(url = %config.core_url, platform = %config.platform_name, "connecting to core");
        let recv_tx_clone = recv_tx.clone();
        let mut builder = ClientBuilder::new(config.core_url.clone())
            .namespace("/ws")
            .opening_header("platform", config.platform_name.clone());
        if let Some(token) = &config.core_token {
            builder = builder.opening_header("Authorization", token.clone());
        }
        // rust-socketio 0.6：connect() 为异步；若你所装版本为同步，去掉 .await
        match builder
            .on("message", move |payload: Payload, _client| {
                let value = match payload {
                    Payload::Text(mut v) => v.drain(..).next().unwrap_or(Value::Null),
                    _ => Value::Null,
                };
                if !value.is_null() && recv_tx_clone.blocking_send(value).is_err() {
                    warn!("core message receiver dropped");
                }
            })
            .connect()
        {
            Ok(socket) => {
                info!("connected to core");
                backoff = config.reconnect_min_interval;
                while let Some(v) = emit_rx.recv().await {
                    if let Err(e) = socket.emit("message", v) {
                        error!(error = %e, "emit to core failed, will reconnect");
                        break;
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "connect to core failed");
            }
        }
        warn!(?backoff, "reconnecting to core");
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(config.reconnect_max_interval);
    }
}
