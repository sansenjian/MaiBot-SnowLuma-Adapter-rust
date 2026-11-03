//! plugin 模式上行（方案 B）：本网关作为 Core plugin_runtime 的 Runner 运行。
//!
//! 入站：inbound.rs 产出的 MessageBase → 转 MessageDict（docs/event-schema.md §7.2）
//!       → host.route_message（Host 侧用 dedupe_key 统一去重，本地不再去重）
//! 出站：Host 调 plugin.invoke_message_gateway，串行 await 每个动作的平台回执，
//!       同一响应带回 message_id_echo 回调（对齐 SnowLuma-Adapter plugin.py 出站流）。
//!
//! ⚠️ method 名 host.route_message / plugin.invoke_message_gateway 已对照
//! SnowLuma-Adapter plugin.py 核实（2026-11-04）。

use anyhow::bail;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::client::{MethodHandler, PluginClient};
use super::envelope::{
    Envelope, METHOD_HEALTH, METHOD_INVOKE_MESSAGE_GATEWAY, METHOD_READY, METHOD_REGISTER,
    METHOD_ROUTE_MESSAGE,
};
use crate::config::Config;

pub async fn run(
    config: Config,
    mut emit_rx: mpsc::Receiver<Value>,
    outbound: Arc<crate::sender::OutboundSender>,
) -> anyhow::Result<()> {
    let mut backoff = config.reconnect_min_interval;
    loop {
        info!(host = %config.plugin_host_address, "connecting to plugin host");
        match PluginClient::connect(&config.plugin_host_address, &config.plugin_session_token).await
        {
            Ok(client) => {
                backoff = config.reconnect_min_interval;
                let client = Arc::new(client);
                if let Err(e) =
                    run_session(&config, client, &mut emit_rx, outbound.clone(), Instant::now()).await
                {
                    warn!(error = %e, "plugin session ended, will reconnect");
                }
            }
            Err(e) => error!(error = %e, "plugin connect/handshake failed"),
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(config.reconnect_max_interval);
    }
}

async fn run_session(
    config: &Config,
    client: Arc<PluginClient>,
    emit_rx: &mut mpsc::Receiver<Value>,
    outbound: Arc<crate::sender::OutboundSender>,
    start: Instant,
) -> anyhow::Result<()> {
    // ---- 注册 MessageGateway 组件（对齐 RegisterPluginPayload / ComponentDeclaration）----
    let register_payload = json!({
        "plugin_id": config.plugin_id,
        "display_name": "MaiBot Rust Gateway",
        "plugin_version": env!("CARGO_PKG_VERSION"),
        "plugin_type": "extension",
        "components": [{
            "name": config.gateway_name,
            "component_type": "MESSAGE_GATEWAY",
            "plugin_id": config.plugin_id,
            "chat_scope": "all",
            "allowed_session": [],
            "metadata": {
                "route_type": "duplex",
                "platform": config.platform_name,
                "protocol": "napcat",
                "handler_name": config.gateway_name,
            }
        }],
        "capabilities_required": [],
    });
    let resp = client
        .request(METHOD_REGISTER, &config.plugin_id, register_payload, 30_000)
        .await?;
    if let Some(err) = resp.error {
        bail!("register_components rejected: {err}");
    }
    info!("message gateway component registered");

    // ---- Host → 本网关 的方法 ----
    // 出站流对齐 SnowLuma-Adapter plugin.py：串行 await 每个动作的平台回执，
    // 在同一 invoke 响应里带回 message_id_echo 回调（internal != external 时）。
    let profile = config.protocol_end.clone();
    let invoke_handler: MethodHandler = Arc::new(move |envelope: Envelope| {
        let outbound = outbound.clone();
        let profile = profile.clone();
        Box::pin(async move {
            let message = envelope
                .payload
                .get("message")
                .or(envelope.payload.get("args").and_then(|a| a.get("message")))
                .cloned()
                .unwrap_or(Value::Null);
            let internal_id = message
                .get("message_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let proxy = crate::outbound::to_outbound_shape(&message);
            match crate::outbound::build_outbound_actions(&proxy, &json!({}), &profile) {
                Ok(actions) => {
                    let action_names: Vec<String> =
                        actions.iter().map(|a| a.action.clone()).collect();
                    let mut external_id = String::new();
                    for action in actions {
                        match outbound.send(action, true).await {
                            Ok(Some(receipt))
                                if receipt.get("status").and_then(Value::as_str) == Some("ok") =>
                            {
                                if external_id.is_empty() {
                                    external_id = receipt
                                        .pointer("/data/message_id")
                                        .or(receipt.pointer("/data/data/message_id"))
                                        .and_then(|v| {
                                            v.as_str().map(str::to_owned).or_else(|| {
                                                v.as_i64()
                                                    .map(|n| n.to_string())
                                                    .or_else(|| v.as_u64().map(|n| n.to_string()))
                                            })
                                        })
                                        .unwrap_or_default();
                                }
                            }
                            Ok(Some(receipt)) => {
                                let wording = receipt
                                    .get("wording")
                                    .or(receipt.get("message"))
                                    .cloned()
                                    .unwrap_or(Value::Null);
                                return envelope.make_response(json!({
                                    "success": false,
                                    "error": format!("action failed: {wording}"),
                                }));
                            }
                            Ok(None) => {
                                return envelope.make_response(json!({
                                    "success": false,
                                    "error": "send receipt timeout",
                                }));
                            }
                            Err(e) => {
                                return envelope
                                    .make_response(json!({ "success": false, "error": e.to_string() }));
                            }
                        }
                    }
                    let mut adapter_callbacks = vec![];
                    if !internal_id.is_empty()
                        && !external_id.is_empty()
                        && internal_id != external_id
                    {
                        adapter_callbacks.push(json!({
                            "name": "message_id_echo",
                            "payload": { "content": { "type": "echo", "echo": internal_id, "actual_id": external_id } },
                        }));
                    }
                    envelope.make_response(json!({
                        "success": true,
                        "external_message_id": if external_id.is_empty() { Value::Null } else { json!(external_id) },
                        "metadata": {
                            "action": action_names.first().cloned().unwrap_or_default(),
                            "actions": action_names,
                            "adapter_callbacks": adapter_callbacks,
                        }
                    }))
                }
                Err(e) => {
                    envelope.make_response(json!({ "success": false, "error": e.to_string() }))
                }
            }
        })
    });
    client.register_method(METHOD_INVOKE_MESSAGE_GATEWAY, invoke_handler);

    let plugin_id = config.plugin_id.clone();
    client.register_method(
        METHOD_HEALTH,
        Arc::new(move |envelope: Envelope| {
            let plugin_id = plugin_id.clone();
            Box::pin(async move {
                envelope.make_response(json!({
                    "healthy": true,
                    "loaded_plugins": [plugin_id],
                    "uptime_ms": start.elapsed().as_millis() as i64,
                }))
            })
        }),
    );
    for lifecycle in ["plugin.shutdown", "plugin.prepare_shutdown"] {
        client.register_method(
            lifecycle,
            Arc::new(|envelope: Envelope| {
                Box::pin(async move { envelope.make_response(json!({ "ack": true })) })
            }),
        );
    }

    // ---- ready ----
    client
        .request(
            METHOD_READY,
            &config.plugin_id,
            json!({
                "loaded_plugins": [{
                    "plugin_id": config.plugin_id,
                    "display_name": "MaiBot Rust Gateway",
                    "plugin_version": env!("CARGO_PKG_VERSION"),
                    "plugin_type": "extension",
                }],
                "failed_plugins": [],
            }),
            30_000,
        )
        .await?;
    info!("runner ready, routing inbound messages");

    // ---- 入站循环 ----
    while let Some(base) = emit_rx.recv().await {
        let (dict, message_id) = to_message_dict(&base, config);
        let payload = json!({
            "gateway_name": config.gateway_name,
            "message": dict,
            "route_metadata": {},
            "external_message_id": message_id,
            "dedupe_key": format!("{}:{}", config.driver_id, message_id),
        });
        match client
            .request(METHOD_ROUTE_MESSAGE, &config.plugin_id, payload, 30_000)
            .await
        {
            Ok(resp) => {
                if let Some(e) = resp.error {
                    warn!(error = %e, "host.route_message rejected");
                }
            }
            Err(e) => {
                warn!(error = %e, "host.route_message failed, reconnecting");
                return Err(e.into());
            }
        }
    }
    Ok(())
}

/// MessageBase → MessageDict（docs/event-schema.md §7.2）。
/// timestamp 对齐 Python `str(float)`（如 "1700000000.0"，用 {:?} 格式化 f64）。
fn to_message_dict(base: &Value, config: &Config) -> (Value, String) {
    let info = base
        .get("message_info")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let message_id = info
        .get("message_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let time_f = info.get("time").and_then(Value::as_f64).unwrap_or(0.0);

    // 段列表：seglist 展开，单段包装
    let seg = base.get("message_segment").cloned().unwrap_or(Value::Null);
    let segments: Vec<Value> = if seg.get("type").and_then(Value::as_str) == Some("seglist") {
        seg.get("data").and_then(Value::as_array).cloned().unwrap_or_default()
    } else if seg.is_null() {
        vec![]
    } else {
        vec![seg]
    };

    // 标志位与纯文本（对齐 build_message_dict 的填充规则）
    let mut plain = String::new();
    let mut is_picture = false;
    let mut is_mentioned = false;
    let mut is_at = false;
    for s in &segments {
        match s.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(t) = s.get("data").and_then(Value::as_str) {
                    if !plain.is_empty() {
                        plain.push(' ');
                    }
                    plain.push_str(t);
                }
            }
            Some("image") => is_picture = true,
            Some("at") => {
                is_at = true;
                is_mentioned = true;
            }
            _ => {}
        }
    }

    let user_info = info.get("user_info").cloned().unwrap_or_else(|| json!({}));
    let group_info = info.get("group_info").cloned();
    let is_group = group_info.as_ref().map(|g| !g.is_null()).unwrap_or(false);

    let self_id = config.self_id.clone().unwrap_or_default();
    let mut additional_config = json!({
        "self_id": self_id,
        "napcat_message_type": if is_group { "group" } else { "private" },
        "platform_io_target_user_id": user_info.get("user_id").cloned().unwrap_or(json!("")),
    });
    if is_group {
        additional_config["platform_io_target_group_id"] =
            group_info.as_ref().unwrap()["group_id"].clone();
    }

    let mut message_info = json!({ "user_info": user_info, "additional_config": additional_config });
    if is_group {
        message_info["group_info"] = group_info.unwrap();
    }

    let dict = json!({
        "message_id": message_id,
        "timestamp": format!("{time_f:?}"),
        "platform": config.platform_name,
        "message_info": message_info,
        "raw_message": segments,
        "is_mentioned": is_mentioned,
        "is_at": is_at,
        "is_emoji": false,
        "is_picture": is_picture,
        "is_command": false,
        "is_notify": false,
        "session_id": "",
        "processed_plain_text": plain,
        "display_message": plain,
    });
    (dict, message_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg() -> Config {
        Config {
            onebot_ws_url: String::new(),
            napcat_token: None,
            protocol_end: crate::profile::ProtocolProfile::snowluma(),
            upstream_mode: crate::config::UpstreamMode::Plugin,
            core_url: String::new(),
            core_token: None,
            plugin_host_address: String::new(),
            plugin_session_token: String::new(),
            plugin_id: "maibot_gateway_rust".into(),
            gateway_name: "napcat_gateway".into(),
            platform_name: "qq".into(),
            driver_id: "legacy.send.qq".into(),
            self_id: Some("100".into()),
            reconnect_min_interval: std::time::Duration::from_secs(1),
            reconnect_max_interval: std::time::Duration::from_secs(60),
        }
    }

    #[test]
    fn message_base_to_dict() {
        let base = json!({
            "message_info": {
                "platform": "qq",
                "message_id": "12345",
                "time": 1700000000.0,
                "user_info": {"platform": "qq", "user_id": "999", "user_nickname": "小明"},
                "group_info": {"platform": "qq", "group_id": "888", "group_name": "群名"}
            },
            "message_segment": {
                "type": "seglist",
                "data": [
                    {"type": "text", "data": "hello"},
                    {"type": "at", "data": {"target_user_id": "100"}},
                    {"type": "image", "data": "https://x/y.png"}
                ]
            }
        });
        let (dict, mid) = to_message_dict(&base, &cfg());
        assert_eq!(mid, "12345");
        assert_eq!(dict["timestamp"], json!("1700000000.0"));
        assert_eq!(dict["message_info"]["additional_config"]["self_id"], json!("100"));
        assert_eq!(dict["message_info"]["additional_config"]["napcat_message_type"], json!("group"));
        assert_eq!(dict["message_info"]["additional_config"]["platform_io_target_group_id"], json!("888"));
        assert_eq!(dict["processed_plain_text"], json!("hello"));
        assert!(dict["is_picture"].as_bool().unwrap());
        assert!(dict["is_mentioned"].as_bool().unwrap());
        assert_eq!(dict["raw_message"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn private_message_no_group_info() {
        let base = json!({
            "message_info": {
                "platform": "qq", "message_id": "1", "time": 1.5,
                "user_info": {"user_id": "42", "user_nickname": "n"}
            },
            "message_segment": {"type": "text", "data": "hi"}
        });
        let (dict, _) = to_message_dict(&base, &cfg());
        assert!(dict["message_info"].get("group_info").is_none());
        assert_eq!(dict["message_info"]["additional_config"]["napcat_message_type"], json!("private"));
        assert_eq!(dict["timestamp"], json!("1.5"));
    }
}
