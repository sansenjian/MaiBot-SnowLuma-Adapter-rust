//! plugin_runtime Envelope 的 serde 模型。
//! 逐字段对齐 Core `src/plugin_runtime/protocol/envelope.py`（2026-11-03 main 分支提取）。
//! 线格式：msgpack 编码的 Envelope 字典，帧头 4 字节大端长度（见 client.rs，帧格式
//! 以 transport/base.py 为准，首次联调核对）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "1.0.0";
pub const DEFAULT_TIMEOUT_MS: i64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
    #[serde(rename = "broadcast")]
    Broadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    /// 单调递增请求 ID（对齐 RequestIdGenerator，从 1 开始）
    pub request_id: i64,
    pub message_type: MessageType,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub plugin_id: String,
    #[serde(default = "now_millis")]
    pub timestamp_ms: i64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
    #[serde(default)]
    pub payload: Value,
    /// 仅 response 携带；{code, message, details}
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

fn default_protocol_version() -> String {
    PROTOCOL_VERSION.to_owned()
}

fn default_timeout_ms() -> i64 {
    DEFAULT_TIMEOUT_MS
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Envelope {
    pub fn new_request(request_id: i64, method: impl Into<String>, plugin_id: impl Into<String>, payload: Value) -> Envelope {
        Envelope {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            request_id,
            message_type: MessageType::Request,
            method: method.into(),
            plugin_id: plugin_id.into(),
            timestamp_ms: now_millis(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            payload,
            error: None,
        }
    }

    pub fn is_request(&self) -> bool {
        self.message_type == MessageType::Request
    }
    pub fn is_response(&self) -> bool {
        self.message_type == MessageType::Response
    }
    pub fn is_broadcast(&self) -> bool {
        self.message_type == MessageType::Broadcast
    }

    /// 对齐 Envelope.make_response。
    pub fn make_response(&self, payload: Value) -> Envelope {
        Envelope {
            protocol_version: self.protocol_version.clone(),
            request_id: self.request_id,
            message_type: MessageType::Response,
            method: self.method.clone(),
            plugin_id: self.plugin_id.clone(),
            timestamp_ms: now_millis(),
            timeout_ms: self.timeout_ms,
            payload,
            error: None,
        }
    }

    /// 对齐 Envelope.make_error_response。
    pub fn make_error_response(&self, code: &str, message: impl Into<String>) -> Envelope {
        Envelope {
            protocol_version: self.protocol_version.clone(),
            request_id: self.request_id,
            message_type: MessageType::Response,
            method: self.method.clone(),
            plugin_id: self.plugin_id.clone(),
            timestamp_ms: now_millis(),
            timeout_ms: self.timeout_ms,
            payload: Value::Null,
            error: Some(serde_json::json!({
                "code": code,
                "message": message.into(),
                "details": {},
            })),
        }
    }
}

// ---- 关键 method 名（来源：envelope.py / rpc_client.py / SDK components.py）----
// 已确认：runner.hello、plugin.register_components、runner.ready、plugin.health
// 推断待联调核对：host.route_message（入站注入，payload 对齐 RouteMessagePayload）
//                plugin.invoke_message_gateway（出站调用，payload 对齐 InvokePayload）
pub const METHOD_HELLO: &str = "runner.hello";
pub const METHOD_REGISTER: &str = "plugin.register_components";
pub const METHOD_READY: &str = "runner.ready";
pub const METHOD_HEALTH: &str = "plugin.health";
pub const METHOD_ROUTE_MESSAGE: &str = "host.route_message";
pub const METHOD_INVOKE_MESSAGE_GATEWAY: &str = "plugin.invoke_message_gateway";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msgpack_roundtrip() {
        let env = Envelope::new_request(1, METHOD_HELLO, "", serde_json::json!({
            "runner_id": "r1", "sdk_version": "1.0.0", "session_token": "t"
        }));
        let bytes = rmp_serde::to_vec(&env).unwrap();
        let back: Envelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.request_id, 1);
        assert_eq!(back.method, METHOD_HELLO);
        assert_eq!(back.protocol_version, "1.0.0");
        assert_eq!(back.payload["runner_id"], serde_json::json!("r1"));
    }

    #[test]
    fn error_response_format() {
        let req = Envelope::new_request(7, "x.y", "p", Value::Null);
        let resp = req.make_error_response("E_METHOD_NOT_ALLOWED", "未注册的方法");
        assert!(resp.is_response());
        assert_eq!(resp.error.unwrap()["code"], serde_json::json!("E_METHOD_NOT_ALLOWED"));
    }
}
