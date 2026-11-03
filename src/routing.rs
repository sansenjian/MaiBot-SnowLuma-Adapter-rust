//! 路由，逐语义对齐 platform_io：
//! - types.py 的 RouteKey（规范化、resolution_order、to_dedupe_scope）
//! - routing.py 的 RouteTable（绑定表 + 从最具体到最宽泛解析）
//! - route_key_factory.py 的 RouteKeyFactory（account_id/scope 字段提取与旧格式兼容链）
//!
//! 验收以 Python 版行为回放为准。

use anyhow::{bail, Result};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// 对齐 types.py RouteKey：platform 小写去空格；account_id/scope 去空格，空串归一为 None。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub platform: String,
    pub account_id: Option<String>,
    pub scope: Option<String>,
}

impl RouteKey {
    pub fn new(
        platform: impl Into<String>,
        account_id: Option<String>,
        scope: Option<String>,
    ) -> Result<Self> {
        let platform = platform.into().trim().to_lowercase();
        let account_id = account_id.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        let scope = scope.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        if platform.is_empty() {
            bail!("RouteKey.platform 不能为空");
        }
        Ok(Self {
            platform,
            account_id,
            scope,
        })
    }

    /// 对齐 resolution_order()：从最具体到最宽泛的回退顺序。
    pub fn resolution_order(&self) -> Vec<RouteKey> {
        let mut keys = vec![self.clone()];
        match (&self.account_id, &self.scope) {
            (Some(a), Some(s)) => {
                keys.push(Self::unchecked(&self.platform, Some(a), None));
                keys.push(Self::unchecked(&self.platform, None, Some(s)));
            }
            (Some(a), None) => keys.push(Self::unchecked(&self.platform, Some(a), None)),
            (None, Some(s)) => keys.push(Self::unchecked(&self.platform, None, Some(s))),
            (None, None) => {}
        }
        let default = Self::unchecked(&self.platform, None, None);
        if !keys.contains(&default) {
            keys.push(default);
        }
        keys
    }

    /// 对齐 to_dedupe_scope()：跨驱动共享的去重作用域键。
    pub fn to_dedupe_scope(&self) -> String {
        format!(
            "{}:{}:{}",
            self.platform,
            self.account_id.as_deref().unwrap_or("*"),
            self.scope.as_deref().unwrap_or("*")
        )
    }

    fn unchecked(platform: &str, account_id: Option<&str>, scope: Option<&str>) -> RouteKey {
        RouteKey {
            platform: platform.to_owned(),
            account_id: account_id.map(str::to_owned),
            scope: scope.map(str::to_owned),
        }
    }
}

/// 对齐 types.py DriverKind。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    Legacy,
    Plugin,
    Local,
}

/// 对齐 types.py RouteBinding。
#[derive(Debug, Clone)]
pub struct RouteBinding {
    pub route_key: RouteKey,
    pub driver_id: String,
    pub driver_kind: DriverKind,
    pub priority: i32,
}

/// 对齐 routing.py RouteTable：只保存绑定，按 resolution_order 解析候选。
#[derive(Default)]
pub struct RouteTable {
    bindings: HashMap<RouteKey, HashMap<String, RouteBinding>>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, binding: RouteBinding) {
        self.bindings
            .entry(binding.route_key.clone())
            .or_default()
            .insert(binding.driver_id.clone(), binding);
    }

    /// driver_id 为 None 时移除该路由键下全部绑定；返回被移除的绑定（按优先级降序）。
    pub fn unbind(&mut self, route_key: &RouteKey, driver_id: Option<&str>) -> Vec<RouteBinding> {
        let Some(binding_map) = self.bindings.get_mut(route_key) else {
            return Vec::new();
        };
        let removed: Vec<RouteBinding> = match driver_id {
            None => {
                let removed: Vec<RouteBinding> = binding_map.drain().map(|(_, v)| v).collect();
                self.bindings.remove(route_key);
                removed
            }
            Some(id) => match binding_map.remove(id) {
                Some(b) => {
                    if binding_map.is_empty() {
                        self.bindings.remove(route_key);
                    }
                    vec![b]
                }
                None => Vec::new(),
            },
        };
        Self::sort_desc(removed)
    }

    pub fn remove_bindings_by_driver(&mut self, driver_id: &str) -> Vec<RouteBinding> {
        let mut removed = Vec::new();
        let mut empty_keys = Vec::new();
        for (route_key, binding_map) in self.bindings.iter_mut() {
            if let Some(b) = binding_map.remove(driver_id) {
                removed.push(b);
            }
            if binding_map.is_empty() {
                empty_keys.push(route_key.clone());
            }
        }
        for key in empty_keys {
            self.bindings.remove(&key);
        }
        Self::sort_desc(removed)
    }

    /// route_key 为 None 时列出全部绑定。
    pub fn list_bindings(&self, route_key: Option<&RouteKey>) -> Vec<RouteBinding> {
        let bindings: Vec<RouteBinding> = match route_key {
            None => self
                .bindings
                .values()
                .flat_map(|m| m.values().cloned())
                .collect(),
            Some(key) => self
                .bindings
                .get(key)
                .map(|m| m.values().cloned().collect())
                .unwrap_or_default(),
        };
        Self::sort_desc(bindings)
    }

    /// 对齐 resolve_bindings()：按从最具体到最宽泛的顺序解析，按 driver_id 去重。
    pub fn resolve_bindings(&self, route_key: &RouteKey) -> Vec<RouteBinding> {
        let mut resolved = Vec::new();
        let mut seen_driver_ids = std::collections::HashSet::new();
        for candidate in route_key.resolution_order() {
            for binding in self.list_bindings(Some(&candidate)) {
                if seen_driver_ids.insert(binding.driver_id.clone()) {
                    resolved.push(binding);
                }
            }
        }
        resolved
    }

    pub fn has_binding_for_driver(&self, route_key: &RouteKey, driver_id: &str) -> bool {
        self.resolve_bindings(route_key)
            .iter()
            .any(|b| b.driver_id == driver_id)
    }

    fn sort_desc(mut bindings: Vec<RouteBinding>) -> Vec<RouteBinding> {
        bindings.sort_by(|a, b| b.priority.cmp(&a.priority));
        bindings
    }
}

// ---- RouteKeyFactory（对齐 route_key_factory.py）----

/// 兼容旧消息归属字段的候选键，按优先级排列。
const ACCOUNT_ID_KEYS: &[&str] = &[
    "platform_io_account_id",
    "account_id",
    "self_id",
    "bot_account",
];
const SCOPE_KEYS: &[&str] = &["platform_io_scope", "route_scope", "adapter_scope", "connection_id"];

/// 对齐 extract_components()：从任意字典提取 account_id / scope。
pub fn extract_components(mapping: Option<&Map<String, Value>>) -> (Option<String>, Option<String>) {
    let Some(map) = mapping else {
        return (None, None);
    };
    (pick_string(map, ACCOUNT_ID_KEYS), pick_string(map, SCOPE_KEYS))
}

/// 对齐 from_message_dict()：优先消息顶层正式 account_id/scope 字段（仅接受字符串），
/// 否则沿旧格式链 message_dict → message_info → additional_config → legacy_metadata 回退。
pub fn route_key_from_message_dict(
    message_dict: &Map<String, Value>,
    legacy_metadata: Option<&Map<String, Value>>,
) -> Result<RouteKey> {
    let platform = message_dict
        .get("platform")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_owned();
    if platform.is_empty() {
        bail!("消息字典缺少有效的 platform 字段，无法构造 RouteKey");
    }

    if message_dict.contains_key("account_id") {
        let account_id = formal_value(message_dict.get("account_id"), "account_id")?;
        let scope = formal_value(message_dict.get("scope"), "scope")?;
        return RouteKey::new(platform, account_id, scope);
    }

    // 旧格式兼容链（Python 版在此打 warning 日志提示适配器迁移）
    let message_info = message_dict
        .get("message_info")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let additional_config = message_info
        .get("additional_config")
        .and_then(Value::as_object);

    let (a1, s1) = extract_components(Some(message_dict));
    let (a2, s2) = extract_components(Some(&message_info));
    let (a3, s3) = extract_components(additional_config);
    let (a4, s4) = extract_components(legacy_metadata);
    RouteKey::new(
        platform,
        a1.or(a2).or(a3).or(a4),
        s1.or(s2).or(s3).or(s4),
    )
}

/// 对齐 _formal_value()：正式字段只接受字符串或 None，空串归一为 None。
fn formal_value(value: Option<&Value>, name: &str) -> Result<Option<String>> {
    match value {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim().to_owned()).filter(|s| !s.is_empty())),
        Some(_) => bail!("消息 {name} 必须是字符串或 None"),
    }
}

/// 对齐 _pick_string()：按优先级取第一个规范化后非空的字符串值。
fn pick_string(mapping: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = mapping.get(*key) {
            if let Some(s) = value.as_str() {
                let normalized = s.trim();
                if !normalized.is_empty() {
                    return Some(normalized.to_owned());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn key(platform: &str, account_id: Option<&str>, scope: Option<&str>) -> RouteKey {
        RouteKey::new(
            platform,
            account_id.map(str::to_owned),
            scope.map(str::to_owned),
        )
        .unwrap()
    }

    #[test]
    fn platform_normalized_and_empty_rejected() {
        let k = key(" QQ ", None, None);
        assert_eq!(k.platform, "qq");
        assert!(RouteKey::new("  ", None, None).is_err());
    }

    #[test]
    fn resolution_order_fallback() {
        let k = key("qq", Some("10001"), Some("napcat"));
        let order = k.resolution_order();
        assert_eq!(order.len(), 4);
        assert_eq!(order[3], key("qq", None, None));
    }

    #[test]
    fn dedupe_scope_format() {
        assert_eq!(key("qq", None, None).to_dedupe_scope(), "qq:*:*");
        assert_eq!(
            key("qq", Some("1"), Some("s")).to_dedupe_scope(),
            "qq:1:s"
        );
    }

    #[test]
    fn formal_fields_win_over_legacy() {
        let dict = json!({
            "platform": "qq",
            "account_id": "10001",
            "self_id": "99999",
        })
        .as_object()
        .unwrap()
        .clone();
        let k = route_key_from_message_dict(&dict, None).unwrap();
        assert_eq!(k.account_id.as_deref(), Some("10001"));
    }

    #[test]
    fn legacy_chain_falls_back_to_self_id() {
        let dict = json!({"platform": "qq", "self_id": " 99999 "})
            .as_object()
            .unwrap()
            .clone();
        let k = route_key_from_message_dict(&dict, None).unwrap();
        assert_eq!(k.account_id.as_deref(), Some("99999"));
    }

    #[test]
    fn missing_platform_errors() {
        let dict = json!({}).as_object().unwrap().clone();
        assert!(route_key_from_message_dict(&dict, None).is_err());
    }
}
