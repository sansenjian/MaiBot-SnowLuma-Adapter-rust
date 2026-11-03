mod config;
mod dedupe;
mod inbound;
mod message;
mod onebot;
mod outbound;
mod routing;
mod upstream;

use serde_json::{json, Value};
use tracing::{debug, info, warn};

/// 端到端管线（方案 A）：
/// NapCat --(OneBot WS)--> onebot::client --原始JSON--> 入站转换(MessageBase) --去重--> upstream(socket.io) --> Core
/// Core --socket.io--> 出站编码(OneBot action) --> onebot::client 写任务 --> NapCat
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "maibot_gateway=info".into()),
        )
        .init();

    let config = config::Config::from_env()?;
    info!(?config, "maibot-gateway starting");

    // NapCat 侧
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<String>(1024);
    let (action_tx, action_rx) = tokio::sync::mpsc::channel::<Value>(256);
    let api = onebot::api::ApiClient::new(action_tx.clone());
    tokio::spawn(onebot::client::run(config.clone(), event_tx, action_rx));

    // Core 侧
    let (core_emit_tx, core_emit_rx) = tokio::sync::mpsc::channel::<Value>(1024);
    let (core_recv_tx, mut core_recv_rx) = tokio::sync::mpsc::channel::<Value>(256);
    tokio::spawn(upstream::run(config.clone(), core_emit_rx, core_recv_tx));

    // 出站管线：Core 下发 → 编码 OneBot 动作 → NapCat（echo 与查询共用计数器）
    let action_tx_out = action_tx.clone();
    let api_out = api.clone();
    tokio::spawn(async move {
        while let Some(msg) = core_recv_rx.recv().await {
            let proxy = to_outbound_shape(&msg);
            match outbound::build_outbound_action(&proxy, &json!({})) {
                Ok(action) => {
                    let frame = json!({
                        "action": action.action,
                        "params": action.params,
                        "echo": api_out.next_echo(),
                    });
                    if action_tx_out.send(frame).await.is_err() {
                        warn!("onebot action channel closed");
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, raw = %msg, "outbound encode failed");
                }
            }
        }
    });

    // 入站管线：NapCat 事件 → 转换 → 去重 → Core
    let mut deduper = dedupe::Deduper::new(
        dedupe::DEFAULT_TTL,
        dedupe::DEFAULT_MAX_ENTRIES,
    )?;
    while let Some(raw) = event_rx.recv().await {
        let parsed: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, raw = %raw, "invalid json frame");
                continue;
            }
        };
        // API 回执（带 echo）：优先喂给查询等待方，未匹配的记日志（出站追踪 TODO）
        if parsed.get("echo").is_some() && parsed.get("status").is_some() {
            if api.resolve_receipt(&parsed) {
                debug!("onebot api receipt (query resolved)");
            } else {
                debug!(receipt = %parsed, "onebot api receipt (untracked)");
            }
            continue;
        }
        if parsed.get("post_type").and_then(Value::as_str) != Some("message") {
            debug!(post_type = ?parsed.get("post_type"), "non-message event ignored");
            continue;
        }
        match inbound::convert_event(&parsed, &config.platform_name, Some(&api)).await {
            Ok(converted) => {
                // 对齐 Core manager.py：dedupe_key = f"{driver_id}:{message_id}"
                let dedupe_key = format!("{}:{}", config.driver_id, converted.message_id);
                if !deduper.mark_seen(&dedupe_key) {
                    info!(dedupe_key = %dedupe_key, "duplicate message dropped");
                    continue;
                }
                if core_emit_tx.send(converted.message).await.is_err() {
                    warn!("core emit channel closed");
                    break;
                }
            }
            Err(e) => {
                warn!(error = %e, raw = %raw, "inbound convert failed");
            }
        }
    }
    Ok(())
}

/// Core 下发的消息可能是 MessageBase（message_segment 为 seglist）或
/// MessageDict（raw_message 为段数组）。统一成出站编码器消费的形状。
fn to_outbound_shape(msg: &Value) -> Value {
    let message_info = msg
        .get("message_info")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let segments = msg
        .get("raw_message")
        .cloned()
        .or_else(|| {
            // MessageBase：message_segment = {type:"seglist", data:[...]} 或单段
            let seg = msg.get("message_segment")?;
            if seg.get("type").and_then(Value::as_str) == Some("seglist") {
                seg.get("data").cloned()
            } else {
                Some(json!([seg]))
            }
        })
        .unwrap_or_else(|| json!([]));
    json!({
        "message_info": message_info,
        "raw_message": segments,
    })
}
