//! IPC protocol over Unix domain socket.
//!
//! Provides a newline-delimited JSON protocol for communication between
//! worktree `mr run` processes and the orchestration daemon.
//!
//! The daemon listens on `.mr/worktrees/daemon.sock`; each worktree
//! `mr run` process connects, sends [`IpcMessage`]s, and receives
//! [`IpcResponse`]s.

// IPC module is defined now but consumed by later tasks (T-005 .. T-018).
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::types::{IpcMessage, IpcResponse};

// ── Constants ───────────────────────────────────────────────────────

/// Default socket file name within `.mr/worktrees/`.
const SOCKET_FILE: &str = "daemon.sock";

// ── Socket path helper ──────────────────────────────────────────────

/// Resolve the daemon socket path for a project root.
///
/// Returns `<root>/.mr/worktrees/daemon.sock`.
#[must_use]
pub fn socket_path(root: &Path) -> PathBuf {
    root.join(".mr").join("worktrees").join(SOCKET_FILE)
}

/// Check whether the daemon socket exists and is connectable.
pub fn is_daemon_reachable(socket: &Path) -> bool {
    UnixStream::connect(socket).is_ok()
}

// ── Client ──────────────────────────────────────────────────────────

/// IPC client for sending messages from a worktree `mr run` process
/// to the orchestration daemon.
///
/// Uses newline-delimited JSON: each message is one JSON object
/// followed by `\n`, and the response is likewise a single JSON line.
pub struct IpcClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl IpcClient {
    /// Connect to the daemon socket at the given path.
    pub fn connect(socket: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket)
            .with_context(|| format!("failed to connect to daemon socket: {}", socket.display()))?;

        let writer = stream
            .try_clone()
            .context("failed to clone unix stream for writer")?;

        Ok(Self {
            reader: BufReader::new(stream),
            writer,
        })
    }

    /// Send a message and receive the daemon's response.
    pub fn send(&mut self, msg: &IpcMessage) -> Result<IpcResponse> {
        // Serialize message as a single JSON line.
        let mut json =
            serde_json::to_string(msg).context("failed to serialize IPC message to JSON")?;
        json.push('\n');

        self.writer
            .write_all(json.as_bytes())
            .context("failed to write IPC message to socket")?;

        self.writer
            .flush()
            .context("failed to flush IPC message to socket")?;

        // Read the response line.
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .context("failed to read IPC response from socket")?;

        serde_json::from_str(line.trim()).context("failed to parse IPC response from JSON")
    }
}

// ── Server ──────────────────────────────────────────────────────────

/// IPC server for the daemon to receive messages from worktree processes.
///
/// Listens on a Unix domain socket and dispatches incoming messages to a
/// user-supplied handler function.
pub struct IpcServer {
    listener: UnixListener,
    path: PathBuf,
}

impl IpcServer {
    /// Create a new server listening on the given socket path.
    ///
    /// Removes any stale socket file before binding.
    pub fn bind(socket: &Path) -> Result<Self> {
        // Remove stale socket file if present (e.g., after a crash).
        if socket.exists() {
            std::fs::remove_file(socket)
                .with_context(|| format!("failed to remove stale socket: {}", socket.display()))?;
        }

        // Ensure the parent directory exists.
        if let Some(parent) = socket.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create socket parent directory: {}",
                    parent.display()
                )
            })?;
        }

        let listener = UnixListener::bind(socket)
            .with_context(|| format!("failed to bind unix socket: {}", socket.display()))?;

        Ok(Self {
            listener,
            path: socket.to_path_buf(),
        })
    }

    /// Accept the next incoming connection and handle all messages on it.
    ///
    /// Reads newline-delimited JSON messages from the connection, passes
    /// each to the `handler`, and writes the returned [`IpcResponse`] back.
    ///
    /// Returns when the client disconnects (EOF) or an I/O error occurs.
    pub fn accept_one<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(IpcMessage) -> IpcResponse,
    {
        let (stream, _addr) = self
            .listener
            .accept()
            .context("failed to accept connection on daemon socket")?;

        Self::handle_connection(stream, &mut handler)
    }

    /// Handle all messages on an established connection.
    fn handle_connection<F>(stream: UnixStream, handler: &mut F) -> Result<()>
    where
        F: FnMut(IpcMessage) -> IpcResponse,
    {
        let mut writer = stream
            .try_clone()
            .context("failed to clone unix stream for response writer")?;

        let reader = BufReader::new(stream);

        for line_result in reader.lines() {
            let line = line_result.context("failed to read line from IPC connection")?;

            if line.trim().is_empty() {
                continue;
            }

            let msg: IpcMessage = serde_json::from_str(&line)
                .with_context(|| format!("failed to parse IPC message: {line}"))?;

            let response = handler(msg);

            let mut resp_json =
                serde_json::to_string(&response).context("failed to serialize IPC response")?;
            resp_json.push('\n');

            writer
                .write_all(resp_json.as_bytes())
                .context("failed to write IPC response")?;

            writer.flush().context("failed to flush IPC response")?;
        }

        Ok(())
    }

    /// Set non-blocking mode on the listener.
    ///
    /// In non-blocking mode, [`Self::accept_one`] returns an error with
    /// [`std::io::ErrorKind::WouldBlock`] when no connection is pending.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.listener
            .set_nonblocking(nonblocking)
            .context("failed to set non-blocking mode on daemon socket")
    }

    /// Path to the socket file.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.path
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Best-effort cleanup of the socket file.
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn temp_socket_path(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("test.sock")
    }

    #[test]
    fn socket_path_resolves_correctly() {
        let path = socket_path(Path::new("/home/user/project"));
        assert_eq!(
            path,
            PathBuf::from("/home/user/project/.mr/worktrees/daemon.sock")
        );
    }

    #[test]
    fn server_binds_and_creates_socket_file() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let server = IpcServer::bind(&sock).unwrap();
        assert!(sock.exists());
        assert_eq!(server.socket_path(), sock);
    }

    #[test]
    fn server_cleans_up_socket_on_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        {
            let _server = IpcServer::bind(&sock).unwrap();
            assert!(sock.exists());
        }

        // After drop, socket file should be removed.
        assert!(!sock.exists());
    }

    #[test]
    fn server_removes_stale_socket_on_bind() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        // Create a first server (creates the socket file).
        {
            let _server = IpcServer::bind(&sock).unwrap();
        }

        // Manually recreate the socket file to simulate a stale socket.
        std::fs::write(&sock, "stale").unwrap();
        assert!(sock.exists());

        // Binding again should succeed after removing the stale file.
        let _server = IpcServer::bind(&sock).unwrap();
        assert!(sock.exists());
    }

    #[test]
    fn client_server_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let server = IpcServer::bind(&sock).unwrap();

        // Spawn a client in a separate thread.
        let sock_clone = sock.clone();
        let client_thread = std::thread::spawn(move || {
            let mut client = IpcClient::connect(&sock_clone).unwrap();

            // Send run_started.
            let resp = client
                .send(&IpcMessage::RunStarted {
                    prd: "PRD-0039".to_string(),
                    wt_id: "wt-001".to_string(),
                    pid: 12345,
                })
                .unwrap();
            assert_eq!(resp.status, "ok");

            // Send task_started.
            let resp = client
                .send(&IpcMessage::TaskStarted {
                    prd: "PRD-0039".to_string(),
                    wt_id: "wt-001".to_string(),
                    task: "T-001".to_string(),
                })
                .unwrap();
            assert_eq!(resp.status, "ok");

            // Send heartbeat_request.
            let resp = client.send(&IpcMessage::HeartbeatRequest).unwrap();
            assert_eq!(resp.status, "ok");

            // Drop the client to close the connection.
            drop(client);
        });

        // Server handles the connection.
        server.accept_one(|_msg| IpcResponse::ok()).unwrap();

        client_thread.join().unwrap();
    }

    #[test]
    fn client_server_error_response() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let server = IpcServer::bind(&sock).unwrap();

        let sock_clone = sock.clone();
        let client_thread = std::thread::spawn(move || {
            let mut client = IpcClient::connect(&sock_clone).unwrap();

            let resp = client.send(&IpcMessage::HeartbeatRequest).unwrap();
            assert_eq!(resp.status, "error");
            assert_eq!(resp.message.as_deref(), Some("not ready"));

            drop(client);
        });

        server
            .accept_one(|_msg| IpcResponse::error("not ready"))
            .unwrap();

        client_thread.join().unwrap();
    }

    #[test]
    fn client_server_all_message_types() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let server = IpcServer::bind(&sock).unwrap();

        let sock_clone = sock.clone();
        let client_thread = std::thread::spawn(move || {
            let mut client = IpcClient::connect(&sock_clone).unwrap();

            let messages = vec![
                IpcMessage::RunStarted {
                    prd: "PRD-0001".to_string(),
                    wt_id: "wt-001".to_string(),
                    pid: 1000,
                },
                IpcMessage::TaskStarted {
                    prd: "PRD-0001".to_string(),
                    wt_id: "wt-001".to_string(),
                    task: "T-001".to_string(),
                },
                IpcMessage::TaskCompleted {
                    prd: "PRD-0001".to_string(),
                    wt_id: "wt-001".to_string(),
                    task: "T-001".to_string(),
                },
                IpcMessage::RunCompleted {
                    prd: "PRD-0001".to_string(),
                    wt_id: "wt-001".to_string(),
                },
                IpcMessage::RunFailed {
                    prd: "PRD-0001".to_string(),
                    wt_id: "wt-001".to_string(),
                    error: "UAT failed".to_string(),
                },
                IpcMessage::HeartbeatRequest,
            ];

            for msg in &messages {
                let resp = client.send(msg).unwrap();
                assert_eq!(resp.status, "ok");
            }

            drop(client);
        });

        let mut received = Vec::new();

        server
            .accept_one(|msg| {
                received.push(msg);
                IpcResponse::ok()
            })
            .unwrap();

        client_thread.join().unwrap();

        assert_eq!(received.len(), 6);
    }

    #[test]
    fn is_daemon_reachable_false_when_no_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = tmp.path().join("nonexistent.sock");

        assert!(!is_daemon_reachable(&sock));
    }

    #[test]
    fn is_daemon_reachable_true_when_listening() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let _server = IpcServer::bind(&sock).unwrap();

        assert!(is_daemon_reachable(&sock));
    }

    #[test]
    fn nonblocking_accept_returns_would_block() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = temp_socket_path(&tmp);

        let server = IpcServer::bind(&sock).unwrap();
        server.set_nonblocking(true).unwrap();

        let result = server.accept_one(|_| IpcResponse::ok());
        assert!(result.is_err());

        // The underlying error should be WouldBlock.
        let root = result.unwrap_err();
        let io_err = root
            .chain()
            .find_map(|e| e.downcast_ref::<std::io::Error>());
        assert!(io_err.is_some());
        assert_eq!(io_err.unwrap().kind(), std::io::ErrorKind::WouldBlock);
    }
}
