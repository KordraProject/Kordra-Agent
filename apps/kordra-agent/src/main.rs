//! Local diagnostic agent. Remote pairing is intentionally not available yet.
use axum::{http::header, response::IntoResponse, routing::get, Json, Router};
use std::{env, net::SocketAddr};

fn validate_listen(address: SocketAddr) -> Result<SocketAddr, &'static str> {
    if !address.ip().is_loopback() {
        return Err("pre-alpha agent must bind to loopback until authenticated pairing is implemented");
    }
    Ok(address)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let address: SocketAddr = env::var("KORDRA_AGENT_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:8081".into()).parse()?;
    let address = validate_listen(address)?;
    let router = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"status":"ok"})) }))
        .route("/v1/info", get(|| async {
            Json(serde_json::json!({"agent":"kordra-agent","version":env!("CARGO_PKG_VERSION"),"paired":false}))
        }))
        .route("/metrics", get(|| async {
            ([(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
                "# HELP kordra_agent_up Agent process is running\n# TYPE kordra_agent_up gauge\nkordra_agent_up 1\n")
                .into_response()
        }));
    tracing::info!("Kordra-Agent listening on {address}");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prohibits_public_bind() {
        assert!(validate_listen("0.0.0.0:8081".parse().unwrap()).is_err());
        assert!(validate_listen("127.0.0.1:8081".parse().unwrap()).is_ok());
        assert!(validate_listen("[::1]:8081".parse().unwrap()).is_ok());
    }
}
