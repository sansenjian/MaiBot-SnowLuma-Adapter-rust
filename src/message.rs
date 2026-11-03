//! maim_message `MessageBase` 的 serde 模型（方案 A 的上行载荷，对齐 message_base.py）。
//!
//! 序列化规则（对齐 to_dict，Rust 侧发送时必须遵守）：
//! - 所有 Optional 字段为 None 时**省略该键**（skip_serializing_if），不输出 null。
//! - group_id 为 None 时调用方应把整个 group_info 置 None。
//! - `Seg.data`：seglist 类型序列化为 Seg 数组，其余类型序列化为字符串
//!   （对齐 Seg.to_dict 的递归规则）。
//!
//! 注意：FormatInfo 默认值 content_format=["text"]、accept_format=["text","image"]
//! 抄自 message_base.py 的字段默认，首次编译前请对照仓库源码复核一遍。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBase {
    pub message_info: BaseMessageInfo,
    pub message_segment: Seg,
    /// Python 侧已标记移除的兼容字段：可接收，发送时勿填充。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaseMessageInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_info: Option<GroupInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_info: Option<UserInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_info: Option<FormatInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_info: Option<TemplateInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_config: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_info: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_info: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "::std::string::String::is_empty")]
    pub group_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_nickname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_cardname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    #[serde(default = "default_content_format", skip_serializing_if = "Vec::is_empty")]
    pub content_format: Vec<String>,
    #[serde(default = "default_accept_format", skip_serializing_if = "Vec::is_empty")]
    pub accept_format: Vec<String>,
}

fn default_content_format() -> Vec<String> {
    vec!["text".to_owned()]
}

fn default_accept_format() -> Vec<String> {
    vec!["text".to_owned(), "image".to_owned()]
}

impl Default for FormatInfo {
    fn default() -> Self {
        Self {
            content_format: default_content_format(),
            accept_format: default_accept_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub template_items: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub template_name: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub template_default: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TemplateInfo {
    fn default() -> Self {
        Self {
            template_items: Default::default(),
            template_name: Default::default(),
            template_default: true,
        }
    }
}

/// 对齐 Seg：type 字段重命名（type 是 Rust 保留字）。
/// data 为字符串（text/image/…）或 Seg 数组（seglist，可嵌套）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seg {
    #[serde(rename = "type")]
    pub seg_type: String,
    pub data: SegData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SegData {
    Text(String),
    SegList(Vec<Seg>),
}

impl Seg {
    pub fn text(text: impl Into<String>) -> Seg {
        Seg {
            seg_type: "text".to_owned(),
            data: SegData::Text(text.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn none_fields_are_omitted() {
        let msg = MessageBase {
            message_info: BaseMessageInfo {
                platform: Some("qq".to_owned()),
                ..Default::default()
            },
            message_segment: Seg::text("hi"),
            raw_message: None,
        };
        let v = serde_json::to_value(&msg).unwrap();
        let info = &v["message_info"];
        assert_eq!(info["platform"], json!("qq"));
        assert!(info.get("group_info").is_none());
        assert!(info.get("message_id").is_none());
        assert!(v.get("raw_message").is_none());
    }

    #[test]
    fn seglist_roundtrip_nested() {
        let seg = Seg {
            seg_type: "seglist".to_owned(),
            data: SegData::SegList(vec![Seg::text("a"), Seg::text("b")]),
        };
        let v = serde_json::to_value(&seg).unwrap();
        assert_eq!(v["data"], json!([{"type": "text", "data": "a"}, {"type": "text", "data": "b"}]));
        let back: Seg = serde_json::from_value(v).unwrap();
        assert!(matches!(back.data, SegData::SegList(ref l) if l.len() == 2));
    }

    #[test]
    fn template_default_serializes_true() {
        let t = TemplateInfo::default();
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["template_default"], json!(true));
    }
}
