use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// OneBot v11 事件信封。字段对齐 maim-message 0.6.8（待 P0 schema 文档落地后逐字段收紧）。
/// 未知字段一律容忍：NapCat/go-cqhttp 系实现存在字段方言差异，反序列化必须不炸。
#[derive(Debug, Clone, Deserialize)]
pub struct OneBotEvent {
    pub time: f64,
    #[serde(default)]
    pub self_id: Option<i64>,
    pub post_type: String,
    /// 其余所有字段（message_type、sub_type、user_id、group_id、raw_message、sender 等）收纳于此
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl OneBotEvent {
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.extra.get(key).and_then(Value::as_str)
    }
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.extra.get(key).and_then(Value::as_i64)
    }
}
