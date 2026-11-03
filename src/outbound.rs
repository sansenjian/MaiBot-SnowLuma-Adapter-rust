//! 出站编码器，逐规则对齐 Adapter `codecs/outbound/`
//!（segment_encoder.py 的段映射 + message_codec.py 的动作选择）。
//!
//! 输入为 Host 侧消息 JSON（message_info / raw_message 段数组 / ...），
//! 输出 `(action, params)` 对应 OneBot `send_*` 动作，经 onebot 客户端的
//! echo 动作通道发往 NapCat。出站追踪（echo → message_id 回执）在 client 侧做。

use anyhow::{bail, Context};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct OutboundAction {
    pub action: String,
    pub params: Value,
}

/// 对齐 NapCatOutboundCodec.build_outbound_action。
pub fn build_outbound_action(message: &Value, route: &Value) -> anyhow::Result<OutboundAction> {
    let message_info = as_map(message.get("message_info"));
    let group_info = as_map(message_info.get("group_info"));
    let additional_config = as_map(message_info.get("additional_config"));
    let raw_message = message.get("raw_message").cloned().unwrap_or(Value::Null);

    let segments = convert_segments(&raw_message);
    let is_forward_message = contains_forward_segment(&raw_message);

    // 群消息：group_info.group_id 优先，回退 additional_config.platform_io_target_group_id
    let target_group_id = first_non_empty_string(&[
        group_info.get("group_id"),
        additional_config.get("platform_io_target_group_id"),
    ]);
    if !target_group_id.is_empty() {
        if is_forward_message {
            let nodes = build_forward_message_segments(&raw_message, &additional_config);
            return Ok(OutboundAction {
                action: "send_group_forward_msg".to_owned(),
                params: json!({"group_id": target_group_id, "message": nodes}),
            });
        }
        return Ok(OutboundAction {
            action: "send_group_msg".to_owned(),
            params: json!({"group_id": target_group_id, "message": segments}),
        });
    }

    // 私聊消息：additional_config 两个候选键 → route.target_user_id
    let target_user_id = first_non_empty_string(&[
        additional_config.get("platform_io_target_user_id"),
        additional_config.get("target_user_id"),
        route.get("target_user_id"),
    ]);
    if target_user_id.is_empty() {
        bail!("出站私聊消息缺少 target_user_id");
    }
    if is_forward_message {
        let nodes = build_forward_message_segments(&raw_message, &additional_config);
        return Ok(OutboundAction {
            action: "send_private_forward_msg".to_owned(),
            params: json!({"message": nodes, "user_id": target_user_id}),
        });
    }
    Ok(OutboundAction {
        action: "send_private_msg".to_owned(),
        params: json!({"message": segments, "user_id": target_user_id}),
    })
}

/// 对齐 convert_segments：逐段分发到具体 builder；未知类型/空结果都有降级占位；
/// 整个列表为空时兜底一个空 text 段。
pub fn convert_segments(raw_message: &Value) -> Vec<Value> {
    let Value::Array(items) = raw_message else {
        return vec![text_segment("")];
    };
    let mut out = Vec::new();
    for item in items {
        let Value::Object(map) = item else { continue };
        let seg_type = py_str(map.get("type")).trim().to_owned();
        let built: Vec<Value> = match seg_type.as_str() {
            "text" => vec![text_segment(py_str(map.get("data")))],
            "at" => build_at_segments(map),
            "reply" => build_reply_segments(map),
            "image" => build_image_segments(map),
            "emoji" => build_emoji_segments(map),
            "voice" => build_voice_segments(map),
            "voiceurl" => build_url_media_segments("record", map.get("data")),
            "face" => build_face_segments(map.get("data")),
            "imageurl" => build_url_media_segments("image", map.get("data")),
            "videourl" => build_url_media_segments("video", map.get("data")),
            "video" => build_video_segments(map),
            "file" => build_file_segments(map.get("data")),
            "music" => build_music_segments(map.get("data")),
            "forward" => build_forward_segments(map),
            "dict" => build_dict_segments(map),
            _ => vec![],
        };
        if built.is_empty() {
            if seg_type.is_empty() {
                out.push(text_segment("[unsupported:unknown]"));
            } else if let Some(placeholder) = empty_segment_fallback(&seg_type) {
                out.push(text_segment(placeholder));
            } else {
                out.push(text_segment(format!("[unsupported:{seg_type}]")));
            }
        } else {
            out.extend(built);
        }
    }
    if out.is_empty() {
        out.push(text_segment(""));
    }
    out
}

// ---- 段 builders（对齐 segment_encoder.py，同名一一对应）----

fn build_at_segments(item: &Map<String, Value>) -> Vec<Value> {
    let data = as_map(item.get("data"));
    let target = py_str(data.get("target_user_id")).trim().to_owned();
    if target.is_empty() {
        return vec![];
    }
    vec![json!({"type": "at", "data": {"qq": target}})]
}

fn build_reply_segments(item: &Map<String, Value>) -> Vec<Value> {
    let data = item.get("data");
    let target = match data {
        Some(Value::Object(m)) => py_str(m.get("target_message_id")),
        other => py_str(other),
    }
    .trim()
    .to_owned();
    if target.is_empty() {
        return vec![];
    }
    vec![json!({"type": "reply", "data": {"id": target}})]
}

fn base64_or_empty(item: &Map<String, Value>) -> String {
    py_str(item.get("binary_data_base64")).trim().to_owned()
}

fn build_image_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if b64.is_empty() {
        return vec![];
    }
    vec![json!({"type": "image", "data": {"file": format!("base64://{b64}"), "sub_type": 0}})]
}

fn build_emoji_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if b64.is_empty() {
        return vec![];
    }
    vec![json!({
        "type": "image",
        "data": {"file": format!("base64://{b64}"), "sub_type": 1, "summary": "[动画表情]"}
    })]
}

fn build_voice_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if !b64.is_empty() {
        return vec![json!({"type": "record", "data": {"file": format!("base64://{b64}")}})];
    }
    build_url_media_segments("record", item.get("data"))
        .into_iter()
        .next()
        .map_or_else(|| vec![text_segment("[voice]")], |v| vec![v])
        .into()
}

fn build_url_media_segments(segment_type: &str, data: Option<&Value>) -> Vec<Value> {
    let file_ref = match data {
        Some(Value::Object(m)) => first_non_empty_string(&[m.get("file"), m.get("url")]),
        other => py_str(other).trim().to_owned(),
    };
    if file_ref.is_empty() {
        return vec![];
    }
    vec![json!({"type": segment_type, "data": {"file": normalize_file_reference(&file_ref)}})]
}

fn build_face_segments(data: Option<&Value>) -> Vec<Value> {
    let face_id = match data {
        Some(Value::Object(m)) => py_str(m.get("id")),
        other => py_str(other),
    }
    .trim()
    .to_owned();
    if face_id.is_empty() {
        return vec![];
    }
    vec![json!({"type": "face", "data": {"id": face_id}})]
}

fn build_video_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if !b64.is_empty() {
        return vec![json!({"type": "video", "data": {"file": format!("base64://{b64}")}})];
    }
    build_url_media_segments("video", item.get("data"))
}

fn build_file_segments(data: Option<&Value>) -> Vec<Value> {
    let segment = match data {
        Some(Value::String(s)) => {
            let f = s.trim();
            if f.is_empty() {
                return vec![];
            }
            json!({"type": "file", "data": {"file": normalize_file_reference(f)}})
        }
        Some(Value::Object(m)) => {
            let file_ref = first_non_empty_string(&[m.get("file"), m.get("path"), m.get("url")]);
            if file_ref.is_empty() {
                return vec![];
            }
            let mut d = json!({"file": normalize_file_reference(&file_ref)});
            for opt in ["name", "thumb"] {
                let v = py_str(m.get(opt)).trim().to_owned();
                if !v.is_empty() {
                    d[opt] = json!(v);
                }
            }
            json!({"type": "file", "data": d})
        }
        _ => return vec![],
    };
    vec![segment]
}

fn build_music_segments(data: Option<&Value>) -> Vec<Value> {
    let segment = match data {
        Some(Value::String(s)) => {
            let id = s.trim();
            if id.is_empty() {
                return vec![];
            }
            json!({"type": "music", "data": {"type": "163", "id": id}})
        }
        Some(Value::Object(m)) => {
            let mut platform = py_str(m.get("type")).trim().to_owned();
            if platform != "163" && platform != "qq" {
                platform = "163".to_owned();
            }
            if platform.is_empty() {
                platform = "163".to_owned();
            }
            let id = py_str(m.get("id")).trim().to_owned();
            if id.is_empty() {
                return vec![];
            }
            json!({"type": "music", "data": {"type": platform, "id": id}})
        }
        _ => return vec![],
    };
    vec![segment]
}

fn build_forward_segments(item: &Map<String, Value>) -> Vec<Value> {
    let Some(Value::Array(nodes)) = item.get("data") else {
        return vec![];
    };
    build_forward_nodes(nodes)
}

/// 对齐 _build_forward_nodes。
fn build_forward_nodes(nodes: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for node in nodes {
        let Value::Object(m) = node else { continue };
        let content = convert_segments(&m.get("content").cloned().unwrap_or(Value::Null));
        out.push(json!({
            "type": "node",
            "data": {
                "name": py_str(m.get("user_nickname")).if_empty("QQ用户"),
                "uin": py_str(m.get("user_id")),
                "content": content,
            }
        }));
    }
    out
}

fn build_dict_segments(item: &Map<String, Value>) -> Vec<Value> {
    let Some(Value::Object(m)) = item.get("data") else {
        return vec![];
    };
    let raw_type = py_str(m.get("type")).trim().to_owned();
    let payload = m.get("data").unwrap_or(&Value::Null);
    let segment = match raw_type.as_str() {
        "file" => build_file_segments(Some(payload)).into_iter().next(),
        "music" => build_music_segments(Some(payload)).into_iter().next(),
        "video" => {
            let mut pseudo = Map::new();
            if let Value::Object(p) = payload {
                pseudo.insert("binary_data_base64".into(), p.get("binary_data_base64").cloned().unwrap_or(Value::Null));
                pseudo.insert("data".into(), payload.clone());
            } else {
                pseudo.insert("data".into(), payload.clone());
            }
            build_video_segments(&pseudo).into_iter().next()
        }
        "face" => build_face_segments(Some(payload)).into_iter().next(),
        "voiceurl" => build_url_media_segments("record", Some(payload)).into_iter().next(),
        "imageurl" => build_url_media_segments("image", Some(payload)).into_iter().next(),
        "videourl" => build_url_media_segments("video", Some(payload)).into_iter().next(),
        t @ ("image" | "record" | "reply" | "at") => {
            let Value::Object(p) = payload else { return vec![] };
            json!({"type": t, "data": p})
        }
        _ => return vec![text_segment(format!("[unsupported:{}]", if raw_type.is_empty() { "dict" } else { &raw_type }))],
    };
    segment.into_iter().collect()
}

// ---- forward 消息打包（对齐 message_codec.py）----

fn contains_forward_segment(raw_message: &Value) -> bool {
    let Value::Array(items) = raw_message else { return false };
    items.iter().any(|i| matches!(i, Value::Object(m) if m.get("type").and_then(Value::as_str) == Some("forward")))
}

/// 对齐 _build_forward_message_segments：连续普通段打包成一个 node，
/// forward 段单独转换后按原顺序拼接。
fn build_forward_message_segments(raw_message: &Value, additional_config: &Map<String, Value>) -> Vec<Value> {
    let Value::Array(items) = raw_message else { return vec![] };
    let self_id = py_str(additional_config.get("self_id")).trim().to_owned();
    let mut out = Vec::new();
    let mut regular: Vec<Value> = Vec::new();
    for item in items {
        let Value::Object(m) = item else { continue };
        if m.get("type").and_then(Value::as_str) == Some("forward") {
            if !regular.is_empty() {
                out.push(regular_forward_node(std::mem::take(&mut regular), &self_id));
            }
            out.extend(convert_segments(&json!([item])));
        } else {
            regular.push(item.clone());
        }
    }
    if !regular.is_empty() {
        out.push(regular_forward_node(regular, &self_id));
    }
    out
}

fn regular_forward_node(regular: Vec<Value>, self_id: &str) -> Value {
    json!({
        "type": "node",
        "data": {"name": "MaiBot", "uin": self_id, "content": convert_segments(&json!(regular))}
    })
}

// ---- 小工具 ----

fn text_segment(text: impl Into<String>) -> Value {
    json!({"type": "text", "data": {"text": text.into()}})
}

/// 对齐 _build_empty_segment_fallback。
fn empty_segment_fallback(seg_type: &str) -> Option<&'static str> {
    Some(match seg_type {
        "emoji" => "[emoji]",
        "face" => "[face]",
        "file" => "[file]",
        "image" | "imageurl" => "[image]",
        "music" => "[music]",
        "video" | "videourl" => "[video]",
        "voice" | "voiceurl" => "[voice]",
        _ => return None,
    })
}

/// 对齐 _normalize_file_reference。
fn normalize_file_reference(reference: &str) -> String {
    if reference.starts_with("base64://")
        || reference.starts_with("file://")
        || reference.starts_with("http://")
        || reference.starts_with("https://")
    {
        reference.to_owned()
    } else {
        format!("file://{reference}")
    }
}

fn as_map(v: Option<&Value>) -> Map<String, Value> {
    match v {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    }
}

/// 对齐 Python str(x)：字符串原样，数字转字符串，其余 JSON 序列化。
fn py_str(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

/// 取候选值中第一个规范化后非空的字符串（对齐 build_outbound_action 的 or 链）。
fn first_non_empty_string(candidates: &[Option<&Value>]) -> String {
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

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}
impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_owned()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_text_message() {
        let msg = json!({
            "message_info": {"group_info": {"group_id": "12345"}},
            "raw_message": [{"type": "text", "data": "hello"}]
        });
        let action = build_outbound_action(&msg, &json!({})).unwrap();
        assert_eq!(action.action, "send_group_msg");
        assert_eq!(action.params["group_id"], json!("12345"));
        assert_eq!(action.params["message"], json!([{"type": "text", "data": {"text": "hello"}}]));
    }

    #[test]
    fn private_requires_target() {
        let msg = json!({"raw_message": [{"type": "text", "data": "hi"}]});
        assert!(build_outbound_action(&msg, &json!({})).is_err());
        let msg2 = json!({
            "message_info": {"additional_config": {"platform_io_target_user_id": "999"}},
            "raw_message": [{"type": "text", "data": "hi"}]
        });
        let action = build_outbound_action(&msg2, &json!({})).unwrap();
        assert_eq!(action.action, "send_private_msg");
        assert_eq!(action.params["user_id"], json!("999"));
    }

    #[test]
    fn unknown_segment_degrades() {
        let segs = convert_segments(&json!([{"type": "hologram", "data": "x"}]));
        assert_eq!(segs, json!([{"type": "text", "data": {"text": "[unsupported:hologram]"}}]));
    }

    #[test]
    fn empty_segment_gets_placeholder() {
        let segs = convert_segments(&json!([{"type": "image", "data": ""}]));
        assert_eq!(segs, json!([{"type": "text", "data": {"text": "[image]"}}]));
    }

    #[test]
    fn file_reference_normalized() {
        assert_eq!(normalize_file_reference("/a/b.png"), "file:///a/b.png");
        assert_eq!(normalize_file_reference("https://x/y.png"), "https://x/y.png");
        assert_eq!(normalize_file_reference("base64://AA"), "base64://AA");
    }

    #[test]
    fn forward_packs_regular_nodes() {
        let msg = json!({
            "message_info": {
                "group_info": {"group_id": "1"},
                "additional_config": {"self_id": "100"}
            },
            "raw_message": [
                {"type": "text", "data": "a"},
                {"type": "forward", "data": [{"user_id": "9", "content": [{"type": "text", "data": "f"}]}]},
                {"type": "text", "data": "b"}
            ]
        });
        let action = build_outbound_action(&msg, &json!({})).unwrap();
        assert_eq!(action.action, "send_group_forward_msg");
        let nodes = action.params["message"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0]["data"]["name"], json!("MaiBot"));
        assert_eq!(nodes[0]["data"]["uin"], json!("100"));
        assert_eq!(nodes[2]["data"]["content"], json!([{"type": "text", "data": {"text": "b"}}]));
    }
}
