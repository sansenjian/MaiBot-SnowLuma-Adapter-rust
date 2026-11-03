use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// NapCat OneBot v11 正向 WS 地址（NapCat 侧配置，默认端口 3001）
    pub onebot_ws_url: String,
    /// NapCat 正向 WS 的 Bearer token（NapCat 侧配置，可选）
    pub napcat_token: Option<String>,
    /// MaiBot Core 的 socket.io 服务端地址（如 http://127.0.0.1:8000）
    pub core_url: String,
    /// Core 侧 enable_token 时配置的 token（握手 Authorization 头，可选）
    pub core_token: Option<String>,
    /// 连接 Core 时携带的 platform 头（每平台一条连接）
    pub platform_name: String,
    /// 本网关驱动的 ID，去重键前缀（对齐 Core manager.py 的 driver_id 约定）
    pub driver_id: String,
    pub reconnect_min_interval: Duration,
    pub reconnect_max_interval: Duration,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            onebot_ws_url: std::env::var("ONEBOT_WS_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:3001".into()),
            napcat_token: std::env::var("NAPCAT_TOKEN").ok().filter(|s| !s.is_empty()),
            core_url: std::env::var("CORE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8000".into()),
            core_token: std::env::var("CORE_TOKEN").ok().filter(|s| !s.is_empty()),
            platform_name: std::env::var("PLATFORM").unwrap_or_else(|_| "qq".into()),
            driver_id: std::env::var("DRIVER_ID")
                .unwrap_or_else(|_| "legacy.send.qq".into()),
            reconnect_min_interval: Duration::from_secs(1),
            reconnect_max_interval: Duration::from_secs(60),
        })
    }
}
