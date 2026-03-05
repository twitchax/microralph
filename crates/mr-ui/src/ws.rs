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
