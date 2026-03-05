//! WebSocket handler for real-time state push to connected clients.
//!
//! Sends the current [`AppState`] snapshot on connection, then streams
//! subsequent updates whenever `state.yaml` or PRDs change on disk.

use std::sync::Arc;

use axum::Extension;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::{RwLock, broadcast};

use crate::types::AppState;

/// Axum handler that upgrades an HTTP request to a WebSocket connection
/// for streaming [`AppState`] updates.
#[allow(clippy::unused_async)] // Axum requires handlers to be async.
pub async fn state_ws_handler(
    ws: WebSocketUpgrade,
    Extension(shared): Extension<Arc<RwLock<AppState>>>,
    Extension(tx): Extension<broadcast::Sender<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_state_ws(socket, shared, tx))
}

/// Manages a single WebSocket connection: sends the initial state snapshot,
/// then forwards broadcast updates until the client disconnects.
async fn handle_state_ws(
    mut socket: WebSocket,
    shared: Arc<RwLock<AppState>>,
    tx: broadcast::Sender<AppState>,
) {
    tracing::info!("WebSocket client connected");

    // Send the current state snapshot so the client has data immediately.
    {
        let state = shared.read().await;

        if let Ok(json) = serde_json::to_string(&*state)
            && socket.send(Message::Text(json.into())).await.is_err()
        {
            return;
        }
    }

    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(state) => {
                        let Ok(json) = serde_json::to_string(&state) else {
                            continue;
                        };

                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged, skipped {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(
                        Ok(Message::Close(_) | Message::Binary(_) | Message::Text(_))
                        | Err(_),
                    )
                    | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                }
            }
        }
    }

    tracing::debug!("WebSocket client disconnected");
}

// ── Log streaming WebSocket ─────────────────────────────────────────

/// Axum handler that upgrades an HTTP request to a WebSocket connection
/// for streaming log file lines from a worktree's `run.log`.
#[allow(clippy::unused_async)] // Axum requires handlers to be async.
pub async fn log_ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Path(wt_id): axum::extract::Path<String>,
    Extension(shared): Extension<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_log_ws(socket, wt_id, shared))
}

/// Manages a single log-streaming WebSocket connection: resolves the log
/// file path from the worktree state, then tails the file and streams
/// new lines to the client until disconnection.
async fn handle_log_ws(mut socket: WebSocket, wt_id: String, shared: Arc<RwLock<AppState>>) {
    tracing::info!(wt_id = %wt_id, "log WebSocket client connected");

    // Resolve the log file path from the current state.
    let log_path = {
        let state = shared.read().await;
        state
            .worktree_state
            .worktrees
            .iter()
            .find(|wt| wt.id == wt_id)
            .and_then(|wt| wt.log_file.clone())
    };

    let Some(log_path) = log_path else {
        let _ = socket
            .send(Message::Text(
                format!("[mr-ui] No log file found for worktree {wt_id}").into(),
            ))
            .await;
        return;
    };

    let path = std::path::PathBuf::from(&log_path);

    // Read existing content first, sending it in chunks.
    let mut pos = 0u64;

    if let Ok(contents) = tokio::fs::read_to_string(&path).await
        && !contents.is_empty()
    {
        // Send existing content as initial batch.
        if socket
            .send(Message::Text(contents.clone().into()))
            .await
            .is_err()
        {
            return;
        }
        pos = contents.len() as u64;
    }

    // Tail loop: poll for new bytes every 200ms.
    let poll_interval = tokio::time::Duration::from_millis(200);
    let mut interval = tokio::time::interval(poll_interval);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let Ok(metadata) = tokio::fs::metadata(&path).await else {
                    continue;
                };

                let file_len = metadata.len();

                if file_len > pos {
                    // Read new bytes from the current position.
                    if let Ok(file) = tokio::fs::File::open(&path).await {
                        use tokio::io::{AsyncReadExt, AsyncSeekExt};

                        let mut file = file;
                        if file.seek(std::io::SeekFrom::Start(pos)).await.is_ok() {
                            let to_read = usize::try_from(file_len - pos).unwrap_or(usize::MAX);
                            let mut buf = vec![0u8; to_read];
                            if let Ok(n) = file.read(&mut buf).await {
                                buf.truncate(n);
                                if let Ok(text) = String::from_utf8(buf)
                                    && !text.is_empty()
                                    && socket.send(Message::Text(text.into())).await.is_err()
                                {
                                    break;
                                }
                                pos += n as u64;
                            }
                        }
                    }
                } else if file_len < pos {
                    // File was truncated (e.g., new run started). Reset.
                    pos = 0;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    _ => break,
                }
            }
        }
    }

    tracing::debug!(wt_id = %wt_id, "log WebSocket client disconnected");
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use std::time::Duration;

    use axum::Router;
    use axum::routing::get;
    use futures_util::StreamExt;
    use tokio::net::TcpListener;

    /// Spins up an Axum server with the `/ws/state` route and returns its address.
    async fn start_test_server() -> (
        std::net::SocketAddr,
        Arc<RwLock<AppState>>,
        broadcast::Sender<AppState>,
    ) {
        let shared: Arc<RwLock<AppState>> = Arc::new(RwLock::new(AppState::default()));
        let (tx, _) = broadcast::channel::<AppState>(16);

        let app = Router::new()
            .route("/ws/state", get(state_ws_handler))
            .layer(Extension(Arc::clone(&shared)))
            .layer(Extension(tx.clone()));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        (addr, shared, tx)
    }

    #[tokio::test]
    async fn ws_sends_initial_state_snapshot() {
        let (addr, _shared, _tx) = start_test_server().await;

        let url = format!("ws://{addr}/ws/state");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // The first message should be the initial state snapshot.
        let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let text = msg.into_text().unwrap();
        let state: AppState = serde_json::from_str(&text).unwrap();
        assert_eq!(state.worktree_state, crate::types::WorktreeState::default());
        assert!(state.prds.is_empty());

        ws.close(None).await.ok();
    }

    #[tokio::test]
    async fn ws_pushes_state_updates_to_clients() {
        let (addr, _shared, tx) = start_test_server().await;

        let url = format!("ws://{addr}/ws/state");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Consume the initial snapshot.
        let _ = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        // Broadcast an updated state.
        let updated = AppState {
            worktree_state: crate::types::WorktreeState {
                version: 42,
                ..Default::default()
            },
            prds: vec![crate::types::PrdSummary {
                id: "PRD-0099".into(),
                title: "Test PRD".into(),
                ..Default::default()
            }],
        };
        tx.send(updated.clone()).unwrap();

        // The WebSocket should push the update.
        let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let text = msg.into_text().unwrap();
        let received: AppState = serde_json::from_str(&text).unwrap();
        assert_eq!(received.worktree_state.version, 42);
        assert_eq!(received.prds.len(), 1);
        assert_eq!(received.prds[0].id, "PRD-0099");

        ws.close(None).await.ok();
    }

    #[tokio::test]
    async fn ws_pushes_multiple_updates_in_order() {
        let (addr, _shared, tx) = start_test_server().await;

        let url = format!("ws://{addr}/ws/state");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Consume initial snapshot.
        let _ = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        // Send two updates in sequence.
        for version in [10, 20] {
            let state = AppState {
                worktree_state: crate::types::WorktreeState {
                    version,
                    ..Default::default()
                },
                ..Default::default()
            };
            tx.send(state).unwrap();
        }

        // Receive both updates and verify ordering.
        let msg1 = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let state1: AppState = serde_json::from_str(&msg1.into_text().unwrap()).unwrap();
        assert_eq!(state1.worktree_state.version, 10);

        let msg2 = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let state2: AppState = serde_json::from_str(&msg2.into_text().unwrap()).unwrap();
        assert_eq!(state2.worktree_state.version, 20);

        ws.close(None).await.ok();
    }
}
