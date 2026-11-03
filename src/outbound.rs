//! 出站编码器 v2，对齐 **MaiBot-SnowLuma-Adapter** `codecs/outbound/message_codec.py`
//!（2026-11-04 提取；协议端画像分支见 profile.rs）。
//!
//! 与 napcat 旧版的核心差异：
//! - 返回**动作列表**（文件可拆为独立上传动作，交错保序）
//! - 发送动作名按画像：Targeted(send_group/private_msg) / Generic(send_msg+message_type)
//! - forward 参数键按画像："message" / "messages"
//! - forward 节点 uin 取消息顶层 account_id（旧版取 additional_config.self_id）
//! - 群/私聊目标解析增加 route.group_id / route.target_group_id / route.user_id 回退
//! - 发送前合并相邻重复 at 段（对齐 _merge_adjacent_mentions，始终启用）
//!
//! 未实现：文本→at 转换（需群成员列表查询，对应 enable_text_at_conversion，TODO）。

use anyhow::bail;
use serde_json::{json, Map, Value};

use crate::profile::{MsgActionStyle, ProtocolProfile};

#[derive(Debug, Clone, PartialEq)]
pub struct OutboundAction {
    pub action: String,
    pub params: Value,
}

/// 对齐 prepare_outbound_actions：返回按序执行的动作列表。
pub fn build_outbound_actions(
    message: &Value,
    route: &Value,
    profile: &ProtocolProfile,
) -> anyhow::Result<Vec<OutboundAction>> {
    let message_info = as_map(message.get("message_info"));
    let group_info = as_map(message_info.get("group_info"));
    let additional_config = as_map(message_info.get("additional_config"));
    let raw_message = message.get("raw_message").cloned().unwrap_or(Value::Null);

    // 目标解析（对齐 _resolve_outbound_target 的完整回退链）
    let target_group_id = first_non_empty_string(&[
        group_info.get("group_id"),
        additional_config.get("platform_io_target_group_id"),
        route.get("group_id"),
        route.get("target_group_id"),
    ]);
    let target_user_id = first_non_empty_string(&[
        additional_config.get("platform_io_target_user_id"),
        additional_config.get("target_user_id"),
        route.get("user_id"),
        route.get("target_user_id"),
    ]);
    if target_group_id.is_empty() && target_user_id.is_empty() {
        bail!("出站消息缺少可用的目标信息（群号或用户ID）");
    }

    // 合并转发消息：单动作
    if contains_forward_segment(&raw_message) {
        let account_id = py_str(message.get("account_id")).trim().to_owned();
        let nodes = build_forward_message_segments(&raw_message, &account_id);
        if nodes.is_empty() {
            bail!("出站消息不包含可发送的内容段");
        }
        let action = if !target_group_id.is_empty() {
            format!("send_{}_forward_msg", if profile.name == "snowluma" { "group" } else { "group" })
        } else {
            "send_private_forward_msg".to_owned()
        };
        let key = profile.forward_param_key;
        let params = if !target_group_id.is_empty() {
            json!({ "group_id": target_group_id, key: nodes })
        } else {
            json!({ "user_id": target_user_id, key: nodes })
        };
        return Ok(vec![OutboundAction { action, params }]);
    }

    // 普通消息：按序生成；画像决定 file 是否拆分为独立上传动作
    let Value::Array(items) = &raw_message else {
        bail!("出站消息不包含可发送的内容段");
    };

    let mut actions = Vec::new();
    let mut pending: Vec<Value> = Vec::new();
    for item in items {
        let Value::Object(m) = item else { continue };
        if profile.split_file_upload && m.get("type").and_then(Value::as_str) == Some("file") {
            if let Some(upload) = build_file_upload_action(m, &target_group_id, &target_user_id) {
                flush_pending(&mut pending, &mut actions, &target_group_id, &target_user_id, profile);
                actions.push(upload);
                continue;
            }
        }
        pending.push(item.clone());
    }
    flush_pending(&mut pending, &mut actions, &target_group_id, &target_user_id, profile);

    if actions.is_empty() {
        bail!("出站消息不包含可发送的内容段");
    }
    Ok(actions)
}

/// 把待发普通段编码为 send 动作并清空缓冲。
fn flush_pending(
    pending: &mut Vec<Value>,
    actions: &mut Vec<OutboundAction>,
    target_group_id: &str,
    target_user_id: &str,
    profile: &ProtocolProfile,
) {
    if pending.is_empty() {
        return;
    }
    let mut segments = convert_segments(&json!(std::mem::take(pending)));
    merge_adjacent_mentions(&mut segments);
    let (action, params) = match profile.msg_action_style {
        MsgActionStyle::Targeted => {
            if !target_group_id.is_empty() {
                (
                    "send_group_msg".to_owned(),
                    json!({ "group_id": target_group_id, "message": segments }),
                )
            } else {
                (
                    "send_private_msg".to_owned(),
                    json!({ "user_id": target_user_id, "message": segments }),
                )
            }
        }
        MsgActionStyle::Generic => {
            let message_type = if !target_group_id.is_empty() { "group" } else { "private" };
            let mut params = json!({ "message_type": message_type, "message": segments });
            if !target_group_id.is_empty() {
                params["group_id"] = json!(target_group_id);
            } else {
                params["user_id"] = json!(target_user_id);
            }
            ("send_msg".to_owned(), params)
        }
    };
    actions.push(OutboundAction { action, params });
}

/// 对齐 _build_file_upload_action：SnowLuma 画像下 file 段拆为独立上传动作。
fn build_file_upload_action(
    item: &Map<String, Value>,
    target_group_id: &str,
    target_user_id: &str,
) -> Option<OutboundAction> {
    let data = item.get("data");
    let (file_ref, name) = match data {
        Some(Value::String(s)) => (s.trim().to_owned(), None),
        Some(Value::Object(m)) => {
            let f = first_non_empty_string(&[m.get("file"), m.get("path"), m.get("url")]);
            let n = py_str(m.get("name")).trim().to_owned();
            (f, if n.is_empty() { None } else { Some(n) })
        }
        _ => (String::new(), None),
    };
    if file_ref.is_empty() {
        return None;
    }
    let (action, mut params) = if !target_group_id.is_empty() {
        (
            "upload_group_file".to_owned(),
            json!({ "group_id": target_group_id, "file": normalize_file_reference(&file_ref) }),
        )
    } else {
        (
            "upload_private_file".to_owned(),
            json!({ "user_id": target_user_id, "file": normalize_file_reference(&file_ref) }),
        )
    };
    if let Some(n) = name {
        params["name"] = json!(n);
    }
    Some(OutboundAction { action, params })
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

/// 对齐 _merge_adjacent_mentions：合并相邻的重复 at 段（始终启用）。
pub fn merge_adjacent_mentions(segments: &mut Vec<Value>) {
    let mut i = 0;
    while i + 1 < segments.len() {
        let same = matches!(
            (&segments[i], &segments[i + 1]),
            (Value::Object(a), Value::Object(b))
                if a.get("type").and_then(Value::as_str) == Some("at")
                && b.get("type").and_then(Value::as_str) == Some("at")
                && a.get("data").and_then(Value::as_object).map(|d| d.get("qq").cloned())
                   == b.get("data").and_then(Value::as_object).map(|d| d.get("qq").cloned())
        );
        if same {
            segments.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

// ---- 段 builders（段映射规则 napcat/snowluma 一致，来自 segment_encoder 同源）----

fn build_at_segments(item: &Map<String, Value>) -> Vec<Value> {
    let data = as_map(item.get("data"));
    let target = py_str(data.get("target_user_id")).trim().to_owned();
    if target.is_empty() {
        return vec![];
    }
    vec![json!({ "type": "at", "data": { "qq": target } })]
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
    vec![json!({ "type": "reply", "data": { "id": target } })]
}

fn base64_or_empty(item: &Map<String, Value>) -> String {
    py_str(item.get("binary_data_base64")).trim().to_owned()
}

fn build_image_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if b64.is_empty() {
        return vec![];
    }
    vec![json!({ "type": "image", "data": { "file": format!("base64://{b64}"), "sub_type": 0 } })]
}

fn build_emoji_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if b64.is_empty() {
        return vec![];
    }
    vec![json!({
        "type": "image",
        "data": { "file": format!("base64://{b64}"), "sub_type": 1, "summary": "[动画表情]" }
    })]
}

fn build_voice_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if !b64.is_empty() {
        return vec![json!({ "type": "record", "data": { "file": format!("base64://{b64}") } })];
    }
    match build_url_media_segments("record", item.get("data")).into_iter().next() {
        Some(v) => vec![v],
        None => vec![],
    }
}

fn build_url_media_segments(segment_type: &str, data: Option<&Value>) -> Vec<Value> {
    let file_ref = match data {
        Some(Value::Object(m)) => first_non_empty_string(&[m.get("file"), m.get("url")]),
        other => py_str(other).trim().to_owned(),
    };
    if file_ref.is_empty() {
        return vec![];
    }
    vec![json!({ "type": segment_type, "data": { "file": normalize_file_reference(&file_ref) } })]
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
    vec![json!({ "type": "face", "data": { "id": face_id } })]
}

fn build_video_segments(item: &Map<String, Value>) -> Vec<Value> {
    let b64 = base64_or_empty(item);
    if !b64.is_empty() {
        return vec![json!({ "type": "video", "data": { "file": format!("base64://{b64}") } })];
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
            json!({ "type": "file", "data": { "file": normalize_file_reference(f) } })
        }
        Some(Value::Object(m)) => {
            let file_ref = first_non_empty_string(&[m.get("file"), m.get("path"), m.get("url")]);
            if file_ref.is_empty() {
                return vec![];
            }
            let mut d = json!({ "file": normalize_file_reference(&file_ref) });
            for opt in ["name", "thumb"] {
                let v = py_str(m.get(opt)).trim().to_owned();
                if !v.is_empty() {
                    d[opt] = json!(v);
                }
            }
            json!({ "type": "file", "data": d })
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
            json!({ "type": "music", "data": { "type": "163", "id": id } })
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
            json!({ "type": "music", "data": { "type": platform, "id": id } })
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
            Some(json!({ "type": t, "data": p }))
        }
        _ => {
            return vec![text_segment(format!("[unsupported:{}]", if raw_type.is_empty() { "dict" } else { &raw_type }))]
        }
    };
    segment.into_iter().collect()
}

// ---- forward 消息打包 ----

fn contains_forward_segment(raw_message: &Value) -> bool {
    let Value::Array(items) = raw_message else { return false };
    items.iter().any(|i| matches!(i, Value::Object(m) if m.get("type").and_then(Value::as_str) == Some("forward")))
}

/// 对齐 _build_forward_message_segments：连续普通段打包成一个 node
///（name="MaiBot"，uin=消息顶层 account_id），forward 段按序拼接。
fn build_forward_message_segments(raw_message: &Value, account_id: &str) -> Vec<Value> {
    let Value::Array(items) = raw_message else { return vec![] };
    let mut out = Vec::new();
    let mut regular: Vec<Value> = Vec::new();
    for item in items {
        let Value::Object(m) = item else { continue };
        if m.get("type").and_then(Value::as_str) == Some("forward") {
            if !regular.is_empty() {
                out.push(regular_forward_node(std::mem::take(&mut regular), account_id));
            }
            out.extend(convert_segments(&json!([item])));
        } else {
            regular.push(item.clone());
        }
    }
    if !regular.is_empty() {
        out.push(regular_forward_node(regular, account_id));
    }
    out
}

fn regular_forward_node(regular: Vec<Value>, account_id: &str) -> Value {
    json!({
        "type": "node",
        "data": { "name": "MaiBot", "uin": account_id, "content": convert_segments(&json!(regular)) }
    })
}

// ---- 小工具 ----

fn text_segment(text: impl Into<String>) -> Value {
    json!({ "type": "text", "data": { "text": text.into() } })
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
pub fn normalize_file_reference(reference: &str) -> String {
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

/// 取候选值中第一个规范化后非空的字符串。
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

/// Core 下发的消息可能是 MessageBase（message_segment 为 seglist）或
/// MessageDict（raw_message 为段数组）。统一成出站编码器消费的形状。
pub fn to_outbound_shape(msg: &Value) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProtocolProfile;

    #[test]
    fn napcat_targeted_group_text() {
        let msg = json!({
            "message_info": {"group_info": {"group_id": "12345"}},
            "raw_message": [{"type": "text", "data": "hello"}]
        });
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::napcat()).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "send_group_msg");
        assert_eq!(actions[0].params["group_id"], json!("12345"));
    }

    #[test]
    fn snowluma_generic_send_msg() {
        let msg = json!({
            "message_info": {"group_info": {"group_id": "12345"}},
            "raw_message": [{"type": "text", "data": "hello"}]
        });
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::snowluma()).unwrap();
        assert_eq!(actions[0].action, "send_msg");
        assert_eq!(actions[0].params["message_type"], json!("group"));
        assert_eq!(actions[0].params["group_id"], json!("12345"));
    }

    #[test]
    fn private_requires_target_with_route_fallback() {
        let msg = json!({"raw_message": [{"type": "text", "data": "hi"}]});
        assert!(build_outbound_actions(&msg, &json!({}), &ProtocolProfile::napcat()).is_err());
        // route.user_id 回退
        let actions = build_outbound_actions(&msg, &json!({"user_id": "777"}), &ProtocolProfile::snowluma()).unwrap();
        assert_eq!(actions[0].action, "send_msg");
        assert_eq!(actions[0].params["message_type"], json!("private"));
        assert_eq!(actions[0].params["user_id"], json!("777"));
    }

    #[test]
    fn snowluma_file_split_upload_order() {
        let msg = json!({
            "message_info": {"group_info": {"group_id": "1"}},
            "raw_message": [
                {"type": "text", "data": "前"},
                {"type": "file", "data": {"file": "/tmp/a.zip", "name": "a.zip"}},
                {"type": "text", "data": "后"}
            ]
        });
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::snowluma()).unwrap();
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].action, "send_msg");
        assert_eq!(actions[1].action, "upload_group_file");
        assert_eq!(actions[1].params["file"], json!("file:///tmp/a.zip"));
        assert_eq!(actions[1].params["name"], json!("a.zip"));
        assert_eq!(actions[2].action, "send_msg");
        // napcat 画像不拆分
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::napcat()).unwrap();
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn snowluma_forward_uses_messages_key() {
        let msg = json!({
            "account_id": "100",
            "message_info": {"group_info": {"group_id": "1"}},
            "raw_message": [
                {"type": "text", "data": "a"},
                {"type": "forward", "data": [{"user_id": "9", "content": [{"type": "text", "data": "f"}]}]}
            ]
        });
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::snowluma()).unwrap();
        assert_eq!(actions[0].action, "send_group_forward_msg");
        assert!(actions[0].params.get("messages").is_some());
        // 节点 uin 来自 account_id
        assert_eq!(actions[0].params["messages"][0]["data"]["uin"], json!("100"));
        // napcat 用 message 键
        let actions = build_outbound_actions(&msg, &json!({}), &ProtocolProfile::napcat()).unwrap();
        assert!(actions[0].params.get("message").is_some());
    }

    #[test]
    fn merge_adjacent_duplicate_mentions() {
        let mut segments = vec![
            json!({"type": "text", "data": {"text": "hi"}}),
            json!({"type": "at", "data": {"qq": "123"}}),
            json!({"type": "at", "data": {"qq": "123"}}),
            json!({"type": "at", "data": {"qq": "456"}}),
        ];
        merge_adjacent_mentions(&mut segments);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[2]["data"]["qq"], json!("456"));
    }

    #[test]
    fn unknown_segment_degrades() {
        let segs = convert_segments(&json!([{"type": "hologram", "data": "x"}]));
        assert_eq!(json!(segs), json!([{"type": "text", "data": {"text": "[unsupported:hologram]"}}]));
    }

    #[test]
    fn file_reference_normalized() {
        assert_eq!(normalize_file_reference("/a/b.png"), "file:///a/b.png");
        assert_eq!(normalize_file_reference("https://x/y.png"), "https://x/y.png");
        assert_eq!(normalize_file_reference("base64://AA"), "base64://AA");
    }
}
