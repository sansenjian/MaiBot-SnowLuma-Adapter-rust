mod config;
mod dedupe;
mod inbound;
mod message;
mod onebot;
mod outbound;
mod plugin;
mod profile;
mod routing;
mod upstream;

use config::UpstreamMode;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

/// 端到端管线：
/// NapCat --(OneBot WS)--> onebot::client --原始JSON--> 入站转换(MessageBase) --> 上行 --> Core
/// Core --> 出站编码(OneBot action) --> onebot::client 写任务 --> NapCat
///
/// 上行二选一（UPSTREAM_MODE）：
/// - socketio（方案 A）：本地去重后 emit MessageBase（Core classic MessageServer）
/// - plugin（方案 B）：MessageBase 转 MessageDict 后 host.route_message
///   （Host 侧用 dedupe_key 去重，本地不再去重）
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

    // Core 侧（按模式分支）
    let (core_emit_tx, core_emit_rx) = tokio::sync::mpsc::channel::<Value>(1024);
    let (core_recv_tx, mut core_recv_rx) = tokio::sync::mpsc::channel::<Value>(256);
    match config.upstream_mode {
        UpstreamMode::Socketio => {
            tokio::spawn(upstream::run(config.clone(), core_emit_rx, core_recv_tx));
        }
        UpstreamMode::Plugin => {
            tokio::spawn(plugin::gateway::run(config.clone(), core_emit_rx, core_recv_tx));
        }
    }

    // 出站管线：Core 下发 → 编码 OneBot 动作列表 → NapCat（echo 与查询共用计数器）
    let action_tx_out = action_tx.clone();
    let api_out = api.clone();
    let outbound_profile = config.protocol_end.clone();
    tokio::spawn(async move {
        while let Some(msg) = core_recv_rx.recv().await {
            let proxy = to_outbound_shape(&msg);
            match outbound::build_outbound_actions(&proxy, &json!({}), &outbound_profile) {
                Ok(actions) => {
                    for action in actions {
                        let frame = json!({
                            "action": action.action,
                            "params": action.params,
                            "echo": api_out.next_echo(),
                        });
                        if action_tx_out.send(frame).await.is_err() {
                            warn!("onebot action channel closed");
                            return;
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, raw = %msg, "outbound encode failed");
                }
            }
        }
    });

    // ---- 事件分派协程：只解析帧并立刻分类 ----
    // 关键：回执必须在此立即 resolve_receipt，绝不能和入站转换在同一个循环里 await
    // ——convert_event 会 await 查询回执，若同循环会自死锁（等自己读的帧）。
    let (inbound_tx, mut inbound_rx) = tokio::sync::mpsc::channel::<Value>(1024);
    let api_dispatch = api.clone();
    tokio::spawn(async move {
        while let Some(raw) = event_rx.recv().await {
            let parsed: Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, raw = %raw, "invalid json frame");
                    continue;
                }
            };
            // API 回执（带 echo）：优先喂给查询等待方，未匹配的记为出站回执
            if parsed.get("echo").is_some() && parsed.get("status").is_some() {
                if api_dispatch.resolve_receipt(&parsed) {
                    debug!("onebot api receipt (query resolved)");
                } else {
                    let actual_id = parsed
                        .pointer("/data/message_id")
                        .or(parsed.pointer("/data/data/message_id"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    info!(echo = ?parsed.get("echo"), message_id = %actual_id, "outbound message sent");
                }
                continue;
            }
            if parsed.get("post_type").and_then(Value::as_str) != Some("message") {
                debug!(post_type = ?parsed.get("post_type"), "non-message event ignored");
                continue;
            }
            if inbound_tx.send(parsed).await.is_err() {
                break;
            }
        }
    });

    // ---- 入站转换协程：可安全 await NapCat 查询回执（回执由分派协程并发处理）----
    let api_convert = api.clone();
    let config_convert = config.clone();
    let emit_tx = core_emit_tx.clone();
    tokio::spawn(async move {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .ok();
        let mut deduper: Option<dedupe::Deduper> =
            match dedupe::Deduper::new(dedupe::DEFAULT_TTL, dedupe::DEFAULT_MAX_ENTRIES) {
                Ok(d) => Some(d),
                Err(e) => {
                    error!(error = %e, "deduper init failed, dedupe disabled");
                    None
                }
            };
        while let Some(parsed) = inbound_rx.recv().await {
            match inbound::convert_event(
                &parsed,
                &config_convert.platform_name,
                Some(&api_convert),
                http.as_ref(),
            )
            .await
            {
                Ok(converted) => {
                    if config_convert.upstream_mode == UpstreamMode::Socketio {
                        // 对齐 Core manager.py：dedupe_key = f"{driver_id}:{message_id}"
                        let dedupe_key =
                            format!("{}:{}", config_convert.driver_id, converted.message_id);
                        if let Some(d) = deduper.as_mut() {
                            if !d.mark_seen(&dedupe_key) {
                                info!(dedupe_key = %dedupe_key, "duplicate message dropped");
                                continue;
                            }
                        }
                    }
                    if emit_tx.send(converted.message).await.is_err() {
                        warn!("core emit channel closed");
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, raw = %parsed, "inbound convert failed");
                }
            }
        }
    });

    // 主线程保持存活（Ctrl-C 优雅退出）
    tokio::signal::ctrl_c().await?;
    Ok(())
}

/// Core 下发的消息可能是 MessageBase（message_segment 为 seglist）或
/// MessageDict（raw_message 为段数组）。统一成出站编码器消费的形状。
fn to_outbound_shape(msg: &Value) -> Value {
    let message_info = msg.get("message_info").cloned().unwrap_or_else(|| json!({}));
    let segments = msg
        .get("raw_message")
        .cloned()
        .or_else(|| {
            let seg = msg.get("message_segment")?;
            if seg.get("type").and_then(Value::as_str) == Some("seglist") {
                seg.get("data").cloned()
            } else {
                Some(json!([seg]))
            }
        })
        .unwrap_or_else(|| json!([]));
    json!({ "message_info": message_info, "raw_message": segments })
}
