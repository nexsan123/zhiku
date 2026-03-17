use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;

/// Shared broadcast channel for WebSocket events.
pub type WsBroadcaster = broadcast::Sender<String>;

/// Start the WebSocket server on port 9600.
/// Returns the broadcast sender so other parts of the app can emit events.
/// Non-blocking -- spawns on its own tokio task via RT-001.
pub fn start_ws_server() -> WsBroadcaster {
    let (tx, _) = broadcast::channel::<String>(128);
    let tx_clone = tx.clone();

    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind("127.0.0.1:9600").await {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind WS server on :9600: {}", e);
                return;
            }
        };

        log::info!("QuantTerminal WebSocket server listening on ws://127.0.0.1:9600");

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    log::warn!("WS accept error: {}", e);
                    continue;
                }
            };

            let mut rx = tx_clone.subscribe();
            log::info!("WS client connected: {}", addr);

            tokio::spawn(async move {
                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        log::warn!("WS handshake failed for {}: {}", addr, e);
                        return;
                    }
                };

                let (mut write, _read) = futures_util::StreamExt::split(ws_stream);

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
