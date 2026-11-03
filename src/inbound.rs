//! 入站转换器：OneBot v11 `message` 事件 → maim_message `MessageBase`（方案 A 上行载荷）。
//!
//! 对齐 Adapter codecs/inbound/message_codec.py。v2 起在有 ApiClient 时补齐以下
//! 依赖 NapCat 查询 API 的增强（单查询 5s 超时，失败按 Python 版语义降级）：
//! - at 段昵称解析（get_group_member_info / get_stranger_info）
//! - reply 段被引消息预览（get_msg）
//! - forward 段 get_forward_msg 查询（无内联节点时）
//!
//! 仍未实现（已知差距）：图片/语音二进制下载转 base64（当前直接用 url）、
//! QQ_FACE 表情表（统一输出 "[表情]"）。

use anyhow::Context;
use serde_json::{json, Value};

use crate::onebot::api::ApiClient;

const QUERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

fn b64encode(bytes: &[u8]) -> String {
    use base64::engine::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub struct InboundMessage {
    /// MessageBase 字典（可直接经 socket.io 发往 Core）
    pub message: Value,
    /// 平台消息 ID（去重键组成部分）
    pub message_id: String,
}

pub async fn convert_event(
    event: &Value,
    platform: &str,
    api: Option<&ApiClient>,
    http: Option<&reqwest::Client>,
) -> anyhow::Result<InboundMessage> {
    let obj = event.as_object().context("OneBot 事件不是 JSON object")?;

    let segments = obj
        .get("message")
        .and_then(Value::as_array)
        .context("OneBot 事件缺少结构化 message 段列表")?;
    let self_id = py_str(obj.get("self_id"));

    let group_id_raw = py_str(obj.get("group_id")).trim().to_owned();
    let is_group = !group_id_raw.is_empty();
    let group_id_num: Option<i64> = group_id_raw.parse().ok();

    let ctx = SegmentContext {
        self_id: self_id.trim().to_owned(),
        is_group,
        group_id_num,
        api,
        http,
    };
    let (converted, is_at) = convert_segments(segments, &ctx).await;

    let message_id = py_str(obj.get("message_id")).trim().to_owned();
    let message_id = if message_id.is_empty() {
        format!("napcat-{}", now_millis())
    } else {
        message_id
    };

    let time = obj
        .get("time")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| now_millis() as f64 / 1000.0);

    let sender = obj
        .get("sender")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let user_id = py_str(obj.get("user_id")).trim().to_owned();
    let cardname = py_str(sender.get("card")).trim().to_owned();
    let nickname = {
        let n = py_str(sender.get("nickname")).trim().to_owned();
        if n.is_empty() { cardname.clone() } else { n }
    };
    let nickname = if nickname.is_empty() { user_id.clone() } else { nickname };

    let mut user_info = json!({ "platform": platform, "user_id": user_id, "user_nickname": nickname });
    if !cardname.is_empty() {
        user_info["user_cardname"] = json!(cardname);
    }

    let group_name = py_str(obj.get("group_name")).trim().to_owned();
    let mut message_info = json!({
        "platform": platform,
        "message_id": message_id,
        "time": time,
        "user_info": user_info,
    });
    if is_group {
        let mut gi = json!({ "platform": platform, "group_id": group_id_raw });
        if !group_name.is_empty() {
            gi["group_name"] = json!(group_name);
        }
        message_info["group_info"] = gi;
    }

    let message_segment = match converted.len() {
        1 => converted.into_iter().next().unwrap(),
        _ => json!({ "type": "seglist", "data": converted }),
    };

    let _ = is_at; // MessageBase 形态不携带 is_at，Core 自行从 seglist 检测
    Ok(InboundMessage {
        message: json!({ "message_info": message_info, "message_segment": message_segment }),
        message_id,
    })
}

struct SegmentContext<'a> {
    self_id: String,
    is_group: bool,
    group_id_num: Option<i64>,
    api: Option<&'a ApiClient>,
    http: Option<&'a reqwest::Client>,
}

async fn convert_segments(segments: &[Value], ctx: &SegmentContext<'_>) -> (Vec<Value>, bool) {
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
                    let mut d = json!({ "target_user_id": target });
                    if let Some((nickname, cardname)) = resolve_at_target(&target, ctx).await {
                        if let Some(n) = nickname {
                            d["target_user_nickname"] = json!(n);
                        }
                        if let Some(c) = cardname {
                            d["target_user_cardname"] = json!(c);
                        }
                    }
                    out.push(json!({ "type": "at", "data": d }));
                    if !ctx.self_id.is_empty() && target == ctx.self_id {
                        is_at = true;
                    }
                }
            }
            "reply" => {
                let target = py_str(data.get("id")).trim().to_owned();
                if !target.is_empty() {
                    out.push(json!({ "type": "reply", "data": build_reply_data(&target, ctx).await }));
                }
            }
            "face" => out.push(text_segment("[表情]")),
            "image" => {
                let url = py_str(data.get("url")).trim().to_owned();
                // 对齐 Python：subType ∉ {0,4,9} 视为表情
                let is_emoji = data
                    .get("subType")
                    .and_then(Value::as_i64)
                    .map(|s| !(s == 0 || s == 4 || s == 9))
                    .unwrap_or(false);
                if !url.is_empty() {
                    match download_base64(ctx.http, &url).await {
                        Some(b64) => out.push(json!({
                            "type": if is_emoji { "emoji" } else { "image" },
                            "data": url,
                            "binary_data_base64": b64,
                        })),
                        None => out.push(text_segment(if is_emoji { "[emoji]" } else { "[image]" })),
                    }
                } else {
                    out.push(text_segment(if is_emoji { "[emoji]" } else { "[image]" }));
                }
            }
            "record" => {
                let file = py_str(data.get("file")).trim().to_owned();
                let b64 = if file.is_empty() {
                    None
                } else {
                    record_base64(&file, ctx).await
                };
                match b64 {
                    Some(b64) if !b64.is_empty() => out.push(json!({
                        "type": "voice",
                        "data": file,
                        "binary_data_base64": b64,
                    })),
                    _ => out.push(text_segment("[voice]")),
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
                let nodes = if let Some(inline) = extract_forward_messages(&data) {
                    Some(inline.clone())
                } else if let Some(id) = data.get("id").and_then(Value::as_str) {
                    query_forward_nodes(id, ctx).await
                } else {
                    None
                };
                match nodes {
                    Some(nodes) if !nodes.is_empty() => {
                        out.push(json!({ "type": "forward", "data": convert_forward_nodes(&nodes) }))
                    }
                    _ => out.push(text_segment("[forward]")),
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

/// 下载图片二进制并转 base64（对齐 Python 的 url 下载）；失败返回 None。
async fn download_base64(http: Option<&reqwest::Client>, url: &str) -> Option<String> {
    let client = http?;
    let resp = tokio::time::timeout(DOWNLOAD_TIMEOUT, client.get(url).send())
        .await
        .ok()?
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = tokio::time::timeout(DOWNLOAD_TIMEOUT, resp.bytes())
        .await
        .ok()?
        .ok()?;
    Some(b64encode(&bytes))
}

/// 对齐 Python record 段：get_record(file) 取 base64，失败返回 None。
async fn record_base64(file: &str, ctx: &SegmentContext<'_>) -> Option<String> {
    let api = ctx.api?;
    let data = tokio::time::timeout(
        QUERY_TIMEOUT,
        api.call("get_record", json!({ "file": file })),
    )
    .await
    .ok()?
    .ok()?;
    data.get("base64").and_then(Value::as_str).map(str::to_owned)
}

/// 对齐 Python 版 at 昵称解析：群消息 get_group_member_info，私聊 get_stranger_info。
/// 查询失败返回 None（Python 版同样降级为不带昵称的 at 段）。
async fn resolve_at_target(target: &str, ctx: &SegmentContext<'_>) -> Option<(Option<String>, Option<String>)> {
    let api = ctx.api?;
    let user_id: i64 = target.parse().ok()?;
    let params = match (ctx.is_group, ctx.group_id_num) {
        (true, Some(gid)) => json!({ "group_id": gid, "user_id": user_id }),
        _ => json!({ "user_id": user_id }),
    };
    let action = if ctx.is_group { "get_group_member_info" } else { "get_stranger_info" };
    let data = tokio::time::timeout(QUERY_TIMEOUT, api.call(action, params))
        .await
        .ok()?
        .ok()?;
    let obj = data.as_object()?;
    let nickname = obj.get("card").and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| obj.get("nickname").and_then(Value::as_str))
        .map(|s| s.trim().to_owned());
    let cardname = obj.get("card").and_then(Value::as_str)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    Some((nickname, cardname))
}

/// 对齐 reply 段被引消息预览：get_msg 提取文本摘要与发送者信息，失败降级为仅 id。
async fn build_reply_data(target_id: &str, ctx: &SegmentContext<'_>) -> Value {
    let mut d = json!({ "target_message_id": target_id });
    let Some(api) = ctx.api else { return d };
    let Ok(Ok(data)) = tokio::time::timeout(
        QUERY_TIMEOUT,
        api.call("get_msg", json!({ "message_id": target_id })),
    )
    .await
    else {
        return d;
    };
    if let Some(content) = data.get("message").and_then(Value::as_array) {
        let preview: String = content
            .iter()
            .filter_map(|s| s.get("data")?.get("text")?.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let preview = preview.trim().to_owned();
        if !preview.is_empty() {
            d["target_message_content"] = json!(preview);
        }
    }
    if let Some(sender) = data.get("sender").and_then(Value::as_object) {
        if let Some(uid) = sender.get("user_id") {
            d["target_sender_user_id"] = json!(py_str(Some(uid)));
        }
        let nickname = sender
            .get("nickname")
            .and_then(Value::as_str)
            .map(|s| s.to_owned())
            .unwrap_or_default();
        if !nickname.is_empty() {
            d["target_sender_nickname"] = json!(nickname);
        }
        let card = sender.get("card").and_then(Value::as_str).unwrap_or("").trim().to_owned();
        if !card.is_empty() {
            d["target_sender_cardname"] = json!(card);
        }
    }
    d
}

/// 对齐 forward 段 get_forward_msg 查询；失败返回 None（降级 [forward] 文本）。
async fn query_forward_nodes(id: &str, ctx: &SegmentContext<'_>) -> Option<Vec<Value>> {
    let api = ctx.api?;
    let data = tokio::time::timeout(
        QUERY_TIMEOUT,
        api.call("get_forward_msg", json!({ "id": id })),
    )
    .await
    .ok()?
    .ok()?;
    data.get("messages")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| data.get("content").and_then(Value::as_array).cloned())
}

/// 对齐 _extract_forward_messages：优先内联 messages/content（不查 API）。
fn extract_forward_messages(data: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    data.get("messages")
        .and_then(Value::as_array)
        .or_else(|| data.get("content").and_then(Value::as_array))
}

/// 对齐 _build_forward_nodes。
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
        // 内联节点的 content 可能出现在节点 data.content（嵌套一层）
        let content_arr = m
            .get("content")
            .or(node_data.get("content"))
            .or(m.get("message"))
            .and_then(Value::as_array);
        let content = content_arr.map(|c| convert_segments_inline(c)).unwrap_or_default();

        let mut n = json!({
            "user_nickname": if nickname.is_empty() { "未知用户".to_owned() } else { nickname },
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

/// 内联 forward 节点的同步转换子集：不发起任何 NapCat 查询
///（at 不带昵称、reply 仅 id、forward 仅内联递归），与 Python 版在无 API
/// 上下文时的降级行为一致。递归出现在同步函数中，无 async 递归问题。
fn convert_segments_inline(segments: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
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
                    out.push(json!({ "type": "at", "data": { "target_user_id": target } }));
                }
            }
            "reply" => {
                let target = py_str(data.get("id")).trim().to_owned();
                if !target.is_empty() {
                    out.push(json!({ "type": "reply", "data": { "target_message_id": target } }));
                }
            }
            "face" => out.push(text_segment("[表情]")),
            "image" => {
                let url = py_str(data.get("url")).trim().to_owned();
                out.push(if !url.is_empty() {
                    json!({ "type": "image", "data": url })
                } else {
                    text_segment("[image]")
                });
            }
            "record" => {
                let url = py_str(data.get("url")).trim().to_owned();
                out.push(if !url.is_empty() {
                    json!({ "type": "voice", "data": url })
                } else {
                    text_segment("[voice]")
                });
            }
            "video" | "file" | "xml" | "share" => out.push(text_segment(&format!("[{seg_type}]"))),
            "forward" => {
                if let Some(nodes) = extract_forward_messages(&data) {
                    let converted = convert_forward_nodes(nodes);
                    if converted.is_empty() {
                        out.push(text_segment("[forward]"));
                    } else {
                        out.push(json!({ "type": "forward", "data": converted }));
                    }
                } else {
                    out.push(text_segment("[forward]"));
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        out.push(text_segment("[empty]"));
    }
    out
}

fn text_segment(text: &str) -> Value {
    json!({ "type": "text", "data": text })
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

    #[tokio::test]
    async fn group_message_with_at() {
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
        let out = convert_event(&event, "qq", None, None).await.unwrap();
        assert_eq!(out.message_id, "12345");
        let info = &out.message["message_info"];
        assert_eq!(info["group_info"]["group_id"], json!("888"));
        assert_eq!(info["user_info"]["user_nickname"], json!("小明"));
        assert_eq!(info["user_info"]["user_cardname"], json!("群名片"));
        let seg = &out.message["message_segment"];
        assert_eq!(seg["type"], json!("seglist"));
        assert_eq!(seg["data"][1]["data"]["target_user_id"], json!("100"));
    }

    #[tokio::test]
    async fn private_single_text_flattens() {
        let event = json!({
            "time": 1.0,
            "post_type": "message",
            "message_type": "private",
            "message_id": 1,
            "user_id": 42,
            "sender": {},
            "message": [{"type": "text", "data": {"text": "hi"}}]
        });
        let out = convert_event(&event, "qq", None, None).await.unwrap();
        assert_eq!(out.message["message_segment"]["type"], json!("text"));
        assert!(out.message["message_info"].get("group_info").is_none());
    }

    #[tokio::test]
    async fn inline_forward_converted_without_api() {
        let event = json!({
            "time": 1.0, "post_type": "message", "message_id": 7, "user_id": 1,
            "group_id": 2, "sender": {"nickname": "n"},
            "message": [{
                "type": "forward",
                "data": {"messages": [
                    {"sender": {"nickname": "甲"}, "content": [{"type": "text", "data": {"text": "内容"}}]}
                ]}
            }]
        });
        let out = convert_event(&event, "qq", None, None).await.unwrap();
        // 单段 flatten：message_segment 即 forward 段本身
        let fwd = &out.message["message_segment"];
        assert_eq!(fwd["type"], json!("forward"));
        assert_eq!(fwd["data"][0]["user_nickname"], json!("甲"));
        assert_eq!(fwd["data"][0]["content"][0]["data"], json!("内容"));
    }

    #[test]
    fn b64encode_matches_standard() {
        assert_eq!(b64encode(b"hello"), "aGVsbG8=");
        assert_eq!(b64encode(b""), "");
    }

    #[tokio::test]
    async fn missing_message_field_errors() {
        let event = json!({"post_type": "message"});
        assert!(convert_event(&event, "qq", None, None).await.is_err());
    }
}
