//! 入站转换器：OneBot v11 `message` 事件 → maim_message `MessageBase`（方案 A 上行载荷）。
//!
//! 对齐 Adapter codecs/inbound/message_codec.py 的**纯同步子集**。以下依赖 NapCat
//! 查询 API 的增强 v1 未实现，属于已知行为差异（待 query client 就绪后补齐）：
//! - at 段昵称解析（get_group_member_info / get_stranger_info）→ nickname/cardname 留空
//! - reply 段被引消息预览（get_msg）→ 仅保留 target_message_id
//! - 图片/语音二进制下载转 base64 → image/voice 段直接使用 url（MessageBase 惯例即 url）
//! - forward 段 get_forward_msg 查询 → 无内联节点时降级为 "[forward]" 文本
//! - QQ_FACE 表情表 → 统一输出 "[表情]"

use anyhow::Context;
use serde_json::{json, Value};

pub struct InboundMessage {
    /// MessageBase 字典（可直接经 socket.io 发往 Core）
    pub message: Value,
    /// 平台消息 ID（去重键组成部分）
    pub message_id: String,
}

pub fn convert_event(event: &Value, platform: &str) -> anyhow::Result<InboundMessage> {
    let obj = event
        .as_object()
        .context("OneBot 事件不是 JSON object")?;

    let segments = obj
        .get("message")
        .and_then(Value::as_array)
        .context("OneBot 事件缺少结构化 message 段列表")?;
    let self_id = py_str(obj.get("self_id"));
    let (converted, is_at) = convert_segments(segments, &self_id);

    let message_id = py_str(obj.get("message_id"))
        .trim()
        .to_owned();
    let message_id = if message_id.is_empty() {
        // 对齐 Python 的 napcat-<uuid4> 回退；v1 用时间戳+自增近似
        format!("napcat-{}", now_millis())
    } else {
        message_id
    };

    let time = obj
        .get("time")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| now_millis() as f64 / 1000.0);

    // user_info：nickname 取 sender.nickname ?? sender.card ?? user_id；cardname 取 sender.card
    let sender = obj.get("sender").and_then(Value::as_object).cloned().unwrap_or_default();
    let user_id = py_str(obj.get("user_id")).trim().to_owned();
    let cardname = py_str(sender.get("card")).trim().to_owned();
    let nickname = py_str(sender.get("nickname"))
        .trim()
        .to_owned();
    let nickname = if nickname.is_empty() {
        cardname.clone()
    } else {
        nickname
    };
    let nickname = if nickname.is_empty() { user_id.clone() } else { nickname };

    let mut user_info = json!({
        "platform": platform,
        "user_id": user_id,
        "user_nickname": nickname,
    });
    if !cardname.is_empty() {
        user_info["user_cardname"] = json!(cardname);
    }

    let group_id = py_str(obj.get("group_id")).trim().to_owned();
    let group_name = py_str(obj.get("group_name")).trim().to_owned();

    let mut message_info = json!({
        "platform": platform,
        "message_id": message_id,
        "time": time,
        "user_info": user_info,
    });
    if !group_id.is_empty() {
        let mut gi = json!({"platform": platform, "group_id": group_id});
        if !group_name.is_empty() {
            gi["group_name"] = json!(group_name);
        }
        message_info["group_info"] = gi;
    }

    let message_segment = match converted.len() {
        1 => converted.into_iter().next().unwrap(),
        _ => json!({"type": "seglist", "data": converted}),
    };

    let _ = is_at; // MessageBase 形态不携带 is_at，Core 自行从 seglist 检测
    Ok(InboundMessage {
        message: json!({
            "message_info": message_info,
            "message_segment": message_segment,
        }),
        message_id,
    })
}

/// 对齐 _convert_incoming_segments 的同步子集，返回 (Host 段列表, 是否 at 到本机器人)。
fn convert_segments(segments: &[Value], self_id: &str) -> (Vec<Value>, bool) {
    let mut out = Vec::new();
    let mut is_at = false;
    for segment in segments {
        let Some(seg) = segment.as_object() else { continue };
        let seg_type = py_str(seg.get("type")).trim().to_owned();
        let data = match seg.get("data") {
            Some(Value::Object(m)) => m.clone(),
            _ => serde_json::Map::new(),
        };
        match seg_type.as_str() {
            "text" => {
                let text = py_str(data.get("text"));
                if !text.is_empty() {
                    out.push(text_segment(&text));
                }
            }
            "at" => {
                let target = py_str(data.get("qq")).trim().to_owned();
                if !target.is_empty() {
                    out.push(json!({
                        "type": "at",
                        "data": {"target_user_id": target}
                    }));
                    if !self_id.is_empty() && target == self_id {
                        is_at = true;
                    }
                }
            }
            "reply" => {
                let target = py_str(data.get("id")).trim().to_owned();
                if !target.is_empty() {
                    out.push(json!({
                        "type": "reply",
                        "data": {"target_message_id": target}
                    }));
                }
            }
            "face" => out.push(text_segment("[表情]")),
            "image" => {
                let url = py_str(data.get("url")).trim().to_owned();
                if !url.is_empty() {
                    out.push(json!({"type": "image", "data": url}));
                } else {
                    out.push(text_segment("[image]"));
                }
            }
            "record" => {
                let url = py_str(data.get("url")).trim().to_owned();
                if !url.is_empty() {
                    out.push(json!({"type": "voice", "data": url}));
                } else {
                    out.push(text_segment("[voice]"));
                }
            }
            "video" => {
                let file_name = py_str(data.get("file")).trim().to_owned();
                let file_size = py_str(data.get("file_size")).trim().to_owned();
                let mut parts = vec![];
                if !file_name.is_empty() {
                    parts.push(format!("文件: {file_name}"));
                }
                if !file_size.is_empty() {
                    parts.push(format!("大小: {file_size}"));
                }
                if parts.is_empty() {
                    out.push(text_segment("[视频]"));
                } else {
                    out.push(text_segment(&format!("[视频] {}", parts.join("，"))));
                }
            }
            "file" => {
                let file_name = py_str(data.get("file").or(data.get("name"))).trim().to_owned();
                let file_size = py_str(data.get("file_size")).trim().to_owned();
                let file_url = py_str(data.get("url")).trim().to_owned();
                let mut text = "[文件]".to_owned();
                let mut parts = vec![];
                if !file_name.is_empty() {
                    parts.push(file_name);
                }
                if !file_size.is_empty() {
                    parts.push(format!("大小: {file_size}"));
                }
                if !parts.is_empty() {
                    text = format!("[文件] {}", parts.join("，"));
                }
                if !file_url.is_empty() {
                    text = format!("{text}，链接: {file_url}");
                }
                out.push(text_segment(&text));
            }
            "forward" => {
                if let Some(nodes) = extract_forward_messages(&data) {
                    let converted_nodes = convert_forward_nodes(nodes);
                    if converted_nodes.is_empty() {
                        out.push(text_segment("[forward]"));
                    } else {
                        out.push(json!({"type": "forward", "data": converted_nodes}));
                    }
                } else {
                    // v1 未实现 get_forward_msg 查询
                    out.push(text_segment("[forward]"));
                }
            }
            "xml" | "share" => out.push(text_segment(&format!("[{seg_type}]"))),
            _ => {}
        }
    }
    if out.is_empty() {
        out.push(text_segment("[unsupported]"));
    }
    (out, is_at)
}

/// 对齐 _extract_forward_messages：优先内联 messages/content（不查 API）。
fn extract_forward_messages(data: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    data.get("messages")
        .and_then(Value::as_array)
        .or_else(|| data.get("content").and_then(Value::as_array))
}

/// 对齐 _build_forward_nodes（内联节点的同步转换）。
fn convert_forward_nodes(nodes: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for node in nodes {
        let Some(m) = node.as_object() else { continue };
        let sender = m
            .get("sender")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let node_data = m.get("data").and_then(Value::as_object).cloned().unwrap_or_default();
        let user_id = first_non_empty(&[
            m.get("user_id").or(node_data.get("user_id")),
            m.get("id"),
            node_data.get("id"),
        ]);
        let nickname = first_non_empty(&[
            sender.get("nickname").or(node_data.get("nickname")),
            sender.get("name").or(node_data.get("name")),
        ]);
        let cardname = first_non_empty(&[sender.get("card").or(node_data.get("card"))]);
        let message_id = first_non_empty(&[m.get("message_id")]);
        let content = m
            .get("content")
            .or(m.get("message"))
            .and_then(Value::as_array)
            .map(|c| convert_segments(c, "").0)
            .unwrap_or_default();

        let mut n = json!({
            "user_nickname": if nickname.is_empty() { "未知用户" } else { nickname },
            "message_id": if message_id.is_empty() { format!("node-{}", now_millis()) } else { message_id },
            "content": if content.is_empty() { vec![text_segment("[empty]")] } else { content },
        });
        if !user_id.is_empty() {
            n["user_id"] = json!(user_id);
        }
        if !cardname.is_empty() {
            n["user_cardname"] = json!(cardname);
        }
        out.push(n);
    }
    out
}

fn text_segment(text: &str) -> Value {
    json!({"type": "text", "data": text})
}

fn py_str(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

fn first_non_empty(candidates: &[Option<&Value>]) -> String {
    for c in candidates {
        if let Some(v) = c {
            let s = py_str(Some(v)).trim().to_owned();
            if !s.is_empty() {
                return s;
            }
        }
    }
    String::new()
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_message_with_at() {
        let event = json!({
            "time": 1700000000.0,
            "self_id": 100,
            "post_type": "message",
            "message_type": "group",
            "message_id": 12345,
            "user_id": 999,
            "group_id": 888,
            "sender": {"nickname": "小明", "card": "群名片"},
            "message": [
                {"type": "text", "data": {"text": "hello"}},
                {"type": "at", "data": {"qq": "100"}}
            ]
        });
        let out = convert_event(&event, "qq").unwrap();
        assert_eq!(out.message_id, "12345");
        let info = &out.message["message_info"];
        assert_eq!(info["group_info"]["group_id"], json!("888"));
        assert_eq!(info["user_info"]["user_nickname"], json!("小明"));
        assert_eq!(info["user_info"]["user_cardname"], json!("群名片"));
        let seg = &out.message["message_segment"];
        assert_eq!(seg["type"], json!("seglist"));
        assert_eq!(seg["data"][1]["data"]["target_user_id"], json!("100"));
    }

    #[test]
    fn private_single_text_flattens() {
        let event = json!({
            "time": 1.0,
            "post_type": "message",
            "message_type": "private",
            "message_id": 1,
            "user_id": 42,
            "sender": {},
            "message": [{"type": "text", "data": {"text": "hi"}}]
        });
        let out = convert_event(&event, "qq").unwrap();
        assert_eq!(out.message["message_segment"]["type"], json!("text"));
        assert!(out.message["message_info"].get("group_info").is_none());
    }

    #[test]
    fn missing_message_field_errors() {
        let event = json!({"post_type": "message"});
        assert!(convert_event(&event, "qq").is_err());
    }
}
