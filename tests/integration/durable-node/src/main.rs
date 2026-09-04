//! A durable ankurah node, started and stopped by the TypeScript integration harness.
//!
//! The harness needs a real Rust peer to talk to: something that stores entities on disk,
//! speaks the websocket wire protocol, and can be started and killed once per test file
//! without leaving state behind. `examples/server` in the support checkout is almost that
//! binary, but it hard-codes its port and storage directory and it writes fake log entries
//! forever. This binary is the same node with those three things fixed: the address and the
//! storage directory come from the command line, nothing writes to the database unless a
//! peer asks it to, and one line on stdout tells the harness which address the node is
//! actually listening on, so tests never have to guess at a sleep.
//!
//! Usage:
//!   ankurah-ts-durable-node --bind 127.0.0.1:0 --storage-dir /tmp/some-empty-dir
//!
//! Port 0 asks the operating system for a free port; the READY line reports the port it
//! chose, which is how the harness avoids picking a port that something else takes first.
//!
//! On stdout it prints exactly one line, `READY <addr:port>`, and nothing else. Everything
//! else — tracing, panics, errors — goes to stderr, so the harness can read stdout as a
//! protocol and report stderr as diagnostics.
//!
//! Unix only: shutdown is driven by SIGTERM and SIGINT.

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use ankurah::{Model, Node, PermissiveAgent};
use ankurah_storage_sled::SledStorageEngine;
use ankurah_websocket_server::WebsocketServer;
use anyhow::{anyhow, Context, Result};
use axum::{routing::get, Router};
use example_model::{Flags, LogEntry};
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, Level};

struct Args {
    bind: String,
    storage_dir: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut bind: Option<String> = None;
    let mut storage_dir: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => bind = Some(args.next().ok_or_else(|| anyhow!("--bind needs an address, e.g. 127.0.0.1:0"))?),
            "--storage-dir" => {
                storage_dir = Some(PathBuf::from(args.next().ok_or_else(|| anyhow!("--storage-dir needs a path"))?))
            }
            other => return Err(anyhow!("unrecognized argument `{other}`; expected --bind <addr:port> --storage-dir <path>")),
        }
    }

    Ok(Args {
        bind: bind.ok_or_else(|| anyhow!("--bind <addr:port> is required"))?,
        storage_dir: storage_dir.ok_or_else(|| anyhow!("--storage-dir <path> is required"))?,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    // stderr, not stdout: stdout carries the READY line and nothing else.
    tracing_subscriber::fmt().with_max_level(Level::INFO).with_writer(std::io::stderr).init();

    let args = parse_args()?;

    std::fs::create_dir_all(&args.storage_dir)
        .with_context(|| format!("could not create storage directory {}", args.storage_dir.display()))?;

    let storage = SledStorageEngine::with_path(args.storage_dir.clone())
        .with_context(|| format!("could not open sled storage at {}", args.storage_dir.display()))?;

    let node = Node::new_durable(Arc::new(storage), PermissiveAgent::new());
    info!("Durable node {} using storage {}", node.id, args.storage_dir.display());

    // A fresh storage directory has no system root, so create one. Reusing a directory
    // across runs keeps whatever root is already there, exactly as examples/server does.
    node.system.wait_loaded().await;
    if node.system.root().is_none() {
        node.system.create().await.context("could not create the system root")?;
    }

    // The example models are what the milestone flow will create and query from both sides.
    // Naming their collections here is what makes this binary the peer for those tests
    // rather than a bare node, and it fails the build if the model crate drifts.
    info!("Example model collections: {}, {}", <Flags as Model>::collection(), <LogEntry as Model>::collection());

    // `WebsocketServer::run` binds its own socket and then serves forever, so it can never
    // tell us which port it got or when it got it. `route_handler` is the same server's
    // other public face — the one meant for hosting the ankurah endpoint inside someone
    // else's axum app — and it lets this binary own the listener. Owning the listener is
    // what makes the READY line trustworthy: a bind that fails fails here and now, and the
    // address we print is the address we are holding, not one we probed and hoped was ours.
    // Mirrors the route and connect-info wiring of `WebsocketServer::run` in
    // ankurah-ts-support/connectors/websocket-server/src/server.rs.
    let server = WebsocketServer::new(node.clone());
    let app = Router::new().route("/ws", get(server.route_handler()));

    let listener = tokio::net::TcpListener::bind(&args.bind)
        .await
        .with_context(|| format!("could not listen on {}", args.bind))?;
    let local_addr = listener.local_addr().context("could not read the address of the listening socket")?;
    info!("Websocket server listening on {}", local_addr);

    // `SmartClientIp` falls back to the peer address of the connection, which only exists
    // when the service is made with connect info.
    let mut serve_task =
        tokio::spawn(async move { axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await });

    println!("READY {local_addr}");
    std::io::stdout().flush()?;

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = sigterm.recv() => info!("SIGTERM received, shutting down"),
        _ = sigint.recv() => info!("SIGINT received, shutting down"),
        finished = &mut serve_task => {
            return Err(match finished {
                Ok(Ok(())) => anyhow!("websocket server stopped unexpectedly"),
                Ok(Err(e)) => anyhow::Error::new(e).context("websocket server failed"),
                Err(e) => anyhow!("websocket server task panicked: {e}"),
            });
        }
    }

    // Stop serving, then drop the last node handle so sled flushes on its way out.
    serve_task.abort();
    drop(node);
    Ok(())
}
