use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;

use crate::services::ai_config;

/// Shared broadcast channel for WebSocket events.
pub type WsBroadcaster = broadcast::Sender<String>;

/// Start the WebSocket server on port 9600.
/// Binds 0.0.0.0 for remote access (cloud deployment).
/// If `qt_api_token` is configured, the client must send the token as its
/// first text message within 10 seconds or the connection is dropped.
/// Returns the broadcast sender so other parts of the app can emit events.
/// Non-blocking -- spawns on its own tokio task via RT-001.
pub fn start_ws_server(app_handle: tauri::AppHandle) -> WsBroadcaster {
    let (tx, _) = broadcast::channel::<String>(128);
    let tx_clone = tx.clone();

    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind("0.0.0.0:9600").await {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind WS server on :9600: {}", e);
                return;
            }
        };

        log::info!("QuantTerminal WebSocket server listening on ws://0.0.0.0:9600");

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    log::warn!("WS accept error: {}", e);
                    continue;
                }
            };

            let mut rx = tx_clone.subscribe();
            let expected_token = ai_config::resolve_qt_token(&app_handle);
            log::info!("WS client connected: {}", addr);

            tokio::spawn(async move {
                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        log::warn!("WS handshake failed for {}: {}", addr, e);
                        return;
                    }
                };

                let (mut write, mut read) = ws_stream.split();

                // Token auth: if configured, require first message to be the token
                if !expected_token.is_empty() {
                    let auth_ok = match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        read.next(),
                    )
                    .await
                    {
                        Ok(Some(Ok(msg))) => msg.into_text().map_or(false, |t| t.trim() == expected_token),
                        _ => false,
                    };

                    if !auth_ok {
                        log::warn!("WS auth rejected for {}: invalid or missing token", addr);
                        let _ = write
                            .send(tokio_tungstenite::tungstenite::Message::Close(None))
                            .await;
                        return;
                    }
                    log::info!("WS client authenticated: {}", addr);
                }

                // Forward broadcast messages to this client
                while let Ok(msg) = rx.recv().await {
                    let ws_msg = tokio_tungstenite::tungstenite::Message::Text(msg.into());
                    if write.send(ws_msg).await.is_err() {
                        log::info!("WS client disconnected: {}", addr);
                        break;
                    }
                }
            });
        }
    });

    tx
}

/// Helper: format a WS event as JSON.
pub fn format_ws_event(event: &str, payload: &impl serde::Serialize) -> String {
    let ts = chrono::Utc::now().to_rfc3339();
    serde_json::json!({
        "event": event,
        "payload": payload,
        "timestamp": ts,
    })
    .to_string()
}
