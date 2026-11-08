//! Local-only device diagnostic bridge for Kordra. Pairing and remote control are not ready.
use axum::{extract::State, http::header, response::IntoResponse, routing::get, Json, Router};
use reqwest::{Client, Url};
use serde_json::{json, Value};
use std::{env, net::{IpAddr, SocketAddr}, time::Duration};

#[derive(Clone)]
struct AgentState {
    client: Client,
    daemon: Url,
    api_token: Option<String>,
}
fn validate_listen(address: SocketAddr) -> Result<SocketAddr, &'static str> {
    if !address.ip().is_loopback() {
        return Err("pre-alpha agent must bind to loopback until authenticated pairing is implemented");
    }
    Ok(address)
}
fn validate_daemon_url(raw: &str) -> Result<Url, &'static str> {
    let url = Url::parse(raw).map_err(|_| "invalid KORDRA_DAEMON_URL")?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("daemon URL must use HTTP(S)");
    }
    if url.username() != "" || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("daemon URL must not include credentials, query or fragment");
    }
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost" || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    });
    if !loopback { return Err("daemon URL must resolve to loopback only"); }
    Ok(url)
}
async fn daemon_status(state: &AgentState) -> bool {
    let Ok(url) = state.daemon.join("/api/v1/health") else { return false };
    let request = state.client.get(url);
    let request = if let Some(token) = &state.api_token { request.bearer_auth(token) } else { request };
    request.send().await.is_ok_and(|response| response.status().is_success())
}
async fn info(State(state): State<AgentState>) -> Json<Value> {
    Json(json!({"agent":"kordra-agent","version":env!("CARGO_PKG_VERSION"),
        "paired":false,"daemon_online":daemon_status(&state).await}))
}
async fn metrics(State(state): State<AgentState>) -> impl IntoResponse {
    let online = u8::from(daemon_status(&state).await);
    let body = format!(
        "# HELP kordra_agent_up Agent is running\n# TYPE kordra_agent_up gauge\nkordra_agent_up 1\n\
         # HELP kordra_agent_daemon_online Local Kordra daemon reachable and healthy\n# TYPE kordra_agent_daemon_online gauge\nkordra_agent_daemon_online {online}\n"
    );
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")], body)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let address: SocketAddr = env::var("KORDRA_AGENT_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:8081".into()).parse()?;
    let address = validate_listen(address)?;
    let daemon = validate_daemon_url(&env::var("KORDRA_DAEMON_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8079".into()))?;
    let client = Client::builder().timeout(Duration::from_secs(4)).build()?;
    let state = AgentState { daemon, client, api_token: env::var("KORDRA_API_TOKEN").ok() };
    let router = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/v1/info", get(info))
        .route("/metrics", get(metrics))
        .with_state(state);
    tracing::info!("Kordra-Agent listening on {address} (loopback only)");
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
    #[test]
    fn local_daemon_only() {
        for url in ["http://127.0.0.1:8079", "https://localhost:8079", "http://[::1]:8079"] {
            assert!(validate_daemon_url(url).is_ok(), "{url}");
        }
        for url in ["http://example.com", "http://8.8.8.8", "http://0.0.0.0:80", "file:///tmp/socket", "http://user:pass@127.0.0.1:8079"] {
            assert!(validate_daemon_url(url).is_err(), "{url}");
        }
    }
}
