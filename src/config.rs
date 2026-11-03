use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamMode {
    /// 方案 A：Core classic socketio MessageServer（MessageBase 载荷）
    Socketio,
    /// 方案 B：Core plugin_runtime（msgpack-RPC over TCP/UDS，MessageDict 载荷）
    Plugin,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// NapCat OneBot v11 正向 WS 地址（NapCat 侧配置，默认端口 3001）
    pub onebot_ws_url: String,
    /// NapCat 正向 WS 的 Bearer token（NapCat 侧配置，可选）
    pub napcat_token: Option<String>,
    pub upstream_mode: UpstreamMode,

    // ---- socketio 模式（方案 A）----
    /// MaiBot Core 的 socket.io 服务端地址（如 http://127.0.0.1:8000）
    pub core_url: String,
    /// Core 侧 enable_token 时配置的 token（握手 Authorization 头，可选）
    pub core_token: Option<String>,

    // ---- plugin 模式（方案 B）----
    /// Core plugin_runtime 的 IPC 地址（如 127.0.0.1:0 由 Core 分配后传入，或 UDS 路径）
    pub plugin_host_address: String,
    /// 握手用一次性会话令牌（Core 启动 Runner 时经环境变量下发）
    pub plugin_session_token: String,
    /// 本网关作为插件注册的 plugin_id
    pub plugin_id: String,
    /// MessageGateway 组件名（注册组件 / route_message 的 gateway_name）
    pub gateway_name: String,

    /// 连接 Core 时携带的 platform（每平台一条连接 / 网关 platform 声明）
    pub platform_name: String,
    /// 本网关代表的机器人 QQ 号（plugin 模式写入 additional_config.self_id）
    pub self_id: Option<String>,
    /// 本网关驱动的 ID，去重键前缀（socketio 模式本地去重；plugin 模式作 dedupe_key 前缀交给 Host）
    pub driver_id: String,
    pub reconnect_min_interval: Duration,
    pub reconnect_max_interval: Duration,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let upstream_mode = match std::env::var("UPSTREAM_MODE").as_deref() {
            Ok("plugin") => UpstreamMode::Plugin,
            _ => UpstreamMode::Socketio,
        };
        Ok(Self {
            onebot_ws_url: std::env::var("ONEBOT_WS_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:3001".into()),
            napcat_token: std::env::var("NAPCAT_TOKEN").ok().filter(|s| !s.is_empty()),
            upstream_mode,
            core_url: std::env::var("CORE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8000".into()),
            core_token: std::env::var("CORE_TOKEN").ok().filter(|s| !s.is_empty()),
            plugin_host_address: std::env::var("PLUGIN_HOST_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:0".into()),
            plugin_session_token: std::env::var("PLUGIN_SESSION_TOKEN")
                .unwrap_or_else(|_| "".into()),
            plugin_id: std::env::var("PLUGIN_ID")
                .unwrap_or_else(|_| "maibot_gateway_rust".into()),
            gateway_name: std::env::var("GATEWAY_NAME")
                .unwrap_or_else(|_| "napcat_gateway".into()),
            platform_name: std::env::var("PLATFORM").unwrap_or_else(|_| "qq".into()),
            self_id: std::env::var("SELF_ID").ok().filter(|s| !s.is_empty()),
            driver_id: std::env::var("DRIVER_ID")
                .unwrap_or_else(|_| "legacy.send.qq".into()),
            reconnect_min_interval: Duration::from_secs(1),
            reconnect_max_interval: Duration::from_secs(60),
        })
    }
}
