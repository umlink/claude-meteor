use crate::AppState;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener as TokioTcpListener;

pub struct ProxyHandle {
    pub shutdown: tokio::task::JoinHandle<()>,
    pub port: u16,
}

pub async fn find_available_port(start: u16) -> u16 {
    let mut port = start;
    for _ in 0..100 {
        if TokioTcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .is_ok()
        {
            return port;
        }
        port += 1;
    }
    start
}

pub async fn start_proxy_server(state: Arc<AppState>, port: u16) -> Result<ProxyHandle, String> {
    let actual_port = find_available_port(port).await;

    let app = Router::new()
        .route(
            "/v1/messages",
            axum::routing::post(crate::proxy::handler::handle_messages),
        )
        .route(
            "/v1/models",
            axum::routing::get(crate::proxy::handler::handle_models),
        )
        .route(
            "/health",
            axum::routing::get(crate::proxy::handler::handle_health),
        )
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(state);

    let listener = TokioTcpListener::bind(format!("127.0.0.1:{}", actual_port))
        .await
        .map_err(|e| format!("Failed to bind port {}: {}", actual_port, e))?;

    tracing::info!("Proxy server listening on 127.0.0.1:{}", actual_port);

    let shutdown_port = actual_port;
    let handle = tokio::spawn(async move {
        let result: Result<(), std::io::Error> = axum::serve(listener, app).await;
        if let Err(e) = result {
            tracing::error!("Proxy server error: {}", e);
        }
        tracing::info!("Proxy server on port {} stopped", shutdown_port);
    });

    Ok(ProxyHandle {
        shutdown: handle,
        port: actual_port,
    })
}
