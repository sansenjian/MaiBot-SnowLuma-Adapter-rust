//! plugin_runtime Runner 端 RPC 客户端，对齐 Core `runner/rpc_client.py`。
//! 传输：TCP（UDS 待补）；帧格式：4 字节大端长度 + msgpack(Envelope)
//! —— 以 transport/base.py 为准，首次联调核对。

use anyhow::Context;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as AsyncMutex;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tracing::{error, info};

use super::envelope::{Envelope, MessageType, METHOD_HELLO};

type HandlerFuture = Pin<Box<dyn Future<Output = Envelope> + Send>>;
pub type MethodHandler = Arc<dyn Fn(Envelope) -> HandlerFuture + Send + Sync>;

pub struct PluginClient {
    write: Arc<AsyncMutex<OwnedWriteHalf>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Envelope>>>>,
    handlers: Arc<Mutex<HashMap<String, MethodHandler>>>,
    request_id: Arc<AtomicI64>,
    runner_id: String,
}

impl PluginClient {
    /// 连接 Host 并完成 runner.hello 握手（对齐 connect_and_handshake）。
    pub async fn connect(host_address: &str, session_token: &str) -> anyhow::Result<PluginClient> {
        let stream = TcpStream::connect(host_address)
            .await
            .with_context(|| format!("connect to plugin host {host_address}"))?;
        let (mut read, write) = stream.into_split();

        let runner_id = uuid_v4();
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Envelope>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let handlers: Arc<Mutex<HashMap<String, MethodHandler>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let request_id = Arc::new(AtomicI64::new(0));

        let client = PluginClient {
            write: Arc::new(AsyncMutex::new(write)),
            pending: pending.clone(),
            handlers: handlers.clone(),
            request_id: request_id.clone(),
            runner_id: runner_id.clone(),
        };

        // runner.hello：10s 超时等待响应
        let hello = Envelope::new_request(
            1,
            METHOD_HELLO,
            "",
            serde_json::json!({
                "runner_id": runner_id,
                "sdk_version": "1.0.0",
                "session_token": session_token,
            }),
        );
        client.send_frame(&hello).await?;
        let hello_bytes = tokio::time::timeout(std::time::Duration::from_secs(10), read_frame(&mut read))
            .await
            .context("runner.hello response timeout")??;
        let hello_resp: Envelope =
            rmp_serde::from_slice(&hello_bytes).context("decode hello response")?;
        let accepted = hello_resp.payload["accepted"].as_bool().unwrap_or(false);
        if !accepted {
            anyhow::bail!(
                "handshake rejected: {}",
                hello_resp.payload["reason"].as_str().unwrap_or("")
            );
        }
        info!(
            host_version = %hello_resp.payload["host_version"].as_str().unwrap_or(""),
            "plugin handshake ok"
        );

        // 请求 ID 从 2 开始（1 用于 hello）
        request_id.store(1, Ordering::SeqCst);

        tokio::spawn(recv_loop(read, client.write.clone(), pending, handlers));
        Ok(client)
    }

    pub fn runner_id(&self) -> &str {
        &self.runner_id
    }

    pub fn register_method(&self, method: &str, handler: MethodHandler) {
        self.handlers.lock().unwrap().insert(method.to_owned(), handler);
    }

    /// 发送请求并等待响应（对齐 send_request）。
    pub async fn request(
        &self,
        method: &str,
        plugin_id: &str,
        payload: Value,
        timeout_ms: u64,
    ) -> anyhow::Result<Envelope> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst) + 1;
        let envelope = Envelope::new_request(id, method, plugin_id, payload);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        if let Err(e) = self.send_frame(&envelope).await {
            self.pending.lock().unwrap().remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => anyhow::bail!("connection closed while waiting {method}"),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                anyhow::bail!("request {method} timeout ({timeout_ms}ms)")
            }
        }
    }

    pub async fn send_frame(&self, envelope: &Envelope) -> anyhow::Result<()> {
        send_frame(&self.write, envelope).await
    }
}

async fn send_frame(write: &Arc<AsyncMutex<OwnedWriteHalf>>, envelope: &Envelope) -> anyhow::Result<()> {
    let payload = rmp_serde::to_vec(envelope).context("msgpack encode envelope")?;
    let mut guard = write.lock().await;
    guard
        .write_u32(payload.len() as u32)
        .await
        .context("write frame header")?;
    guard.write_all(&payload).await.context("write frame body")?;
    Ok(())
}

/// 读循环：分派 response / request / broadcast（对齐 _recv_loop）。
async fn recv_loop(
    mut read: OwnedReadHalf,
    write: Arc<AsyncMutex<OwnedWriteHalf>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Envelope>>>>,
    handlers: Arc<Mutex<HashMap<String, MethodHandler>>>,
) {
    loop {
        let frame = match read_frame(&mut read).await {
            Ok(f) => f,
            Err(e) => {
                info!("plugin host connection closed: {e}");
                break;
            }
        };
        let envelope: Envelope = match rmp_serde::from_slice(&frame) {
            Ok(e) => e,
            Err(e) => {
                error!("decode envelope failed: {e}");
                continue;
            }
        };
        if envelope.is_response() {
            let tx = pending.lock().unwrap().remove(&envelope.request_id);
            if let Some(tx) = tx {
                let _ = tx.send(envelope);
            }
        } else if envelope.is_request() {
            let handler = handlers.lock().unwrap().get(&envelope.method).cloned();
            let Some(handler) = handler else {
                // 对齐 _handle_request：未注册方法回 E_METHOD_NOT_ALLOWED
                let resp = envelope.make_error_response("E_METHOD_NOT_ALLOWED", "method not allowed");
                if let Err(e) = send_frame(&write, &resp).await {
                    error!("send method-not-allowed response failed: {e}");
                    break;
                }
                continue;
            };
            let write = write.clone();
            tokio::spawn(async move {
                let response = handler(envelope).await;
                if let Err(e) = send_frame(&write, &response).await {
                    error!("send response frame failed: {e}");
                }
            });
        } else if envelope.is_broadcast() {
            let handler = handlers.lock().unwrap().get(&envelope.method).cloned();
            if let Some(handler) = handler {
                tokio::spawn(async move {
                    let _ = handler(envelope).await;
                });
            }
        }
    }
    // 连接断开：唤醒所有等待方
    for (_, tx) in pending.lock().unwrap().drain() {
        let _ = tx.send(Envelope {
            protocol_version: super::envelope::PROTOCOL_VERSION.to_owned(),
            request_id: -1,
            message_type: MessageType::Response,
            method: String::new(),
            plugin_id: String::new(),
            timestamp_ms: 0,
            timeout_ms: 0,
            payload: Value::Null,
            error: Some(
                serde_json::json!({"code": "E_UNKNOWN", "message": "connection closed", "details": {}}),
            ),
        });
    }
}

async fn read_frame(read: &mut OwnedReadHalf) -> anyhow::Result<Vec<u8>> {
    let len = read.read_u32().await.context("read frame header")? as usize;
    let mut buf = vec![0u8; len];
    read.read_exact(&mut buf).await.context("read frame body")?;
    Ok(buf)
}

fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    bytes[..8].copy_from_slice(&nanos.to_le_bytes());
    let ctr = std::sync::atomic::AtomicU64::new(0);
    bytes[8..].copy_from_slice(&ctr.fetch_add(1, Ordering::SeqCst).to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
