use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tracing::{error, info, warn};

use crate::config::Config;

/// 正向 WS 连接 NapCat（OneBot v11），断线指数退避自动重连。
///
/// - 入站：文本帧原样经 `event_tx` 送出（事件与 API 回执都会到，回执带 echo 字段，
///   由消费方区分）。
/// - 出站：写任务从 `action_rx` 取动作帧（`{"action", "params", "echo"}`）发往 NapCat，
///   echo 由发送方填入并负责关联回执（出站追踪）。
pub async fn run(
    config: Config,
    event_tx: mpsc::Sender<String>,
    mut action_rx: mpsc::Receiver<Value>,
) -> anyhow::Result<()> {
    let mut backoff = config.reconnect_min_interval;
    loop {
        info!(url = %config.onebot_ws_url, "connecting to napcat");
        let connect_result: anyhow::Result<_> = (|| {
            let mut request = config
                .onebot_ws_url
                .clone()
                .into_client_request()
                .map_err(anyhow::Error::msg)?;
            if let Some(token) = &config.napcat_token {
                request
                    .headers_mut()
                    .insert("Authorization", format!("Bearer {token}").parse()?);
            }
            Ok(request)
        })();
        match connect_result {
            Ok(request) => match connect_async(request).await {
                Ok((ws, _resp)) => {
                    info!("connected to napcat");
                    backoff = config.reconnect_min_interval;
                    let (mut write, mut read) = ws.split();

                    let mut writer = tokio::spawn(async move {
                        while let Some(frame) = action_rx.recv().await {
                            if write.send(frame.to_string().into()).await.is_err() {
                                warn!("napcat ws send failed, writer exiting");
                                break;
                            }
                        }
                    });

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if event_tx.send(text).await.is_err() {
                                    warn!("event receiver dropped, exiting client loop");
                                    writer.abort();
                                    return Ok(());
                                }
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                                warn!("napcat closed the connection");
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "websocket error");
                                break;
                            }
                            _ => {}
                        }
                    }
                    writer.abort();
                }
                Err(e) => {
                    error!(error = %e, "connect to napcat failed");
                }
            },
            Err(e) => {
                error!(error = %e, "invalid napcat ws url");
            }
        }
        warn!(?backoff, "reconnecting to napcat");
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(config.reconnect_max_interval);
    }
}
