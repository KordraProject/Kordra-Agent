//! Local-only diagnostics for Kordra. Remote pairing and management are not enabled.
use axum::{extract::State, http::header, response::IntoResponse, routing::get, Json, Router};
use reqwest::{redirect::Policy, Client, Url};
use serde_json::{json, Value};
use std::{env, net::{IpAddr, SocketAddr}, time::Duration};

#[derive(Clone)]
struct AgentState {
    client: Client,
    daemon: Url,
    api_token: Option<String>,
}
#[derive(Clone, Debug)]
struct DaemonSnapshot {
    online: bool,
    version: Option<String>,
    statistics_available: bool,
    active_tasks: Option<u64>,
    completed_tasks: Option<u64>,
    failed_tasks: Option<u64>,
    downloaded_bytes: Option<u64>,
    uploaded_bytes: Option<u64>,
}
impl DaemonSnapshot {
    fn offline() -> Self {
        Self {
            online: false, version: None, statistics_available: false,
            active_tasks: None, completed_tasks: None, failed_tasks: None,
            downloaded_bytes: None, uploaded_bytes: None,
        }
    }
    fn as_json(&self) -> Value {
        json!({
            "online": self.online,
            "api_version": if self.online { Some("v1") } else { None },
            "daemon_version": self.version,
            "statistics_available": self.statistics_available,
            "tasks": {
                "active": self.active_tasks,
                "completed": self.completed_tasks,
                "failed": self.failed_tasks
            },
            "traffic": {
                "downloaded_bytes": self.downloaded_bytes,
                "uploaded_bytes": self.uploaded_bytes
            }
        })
    }
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
    if url.username() != "" || url.password().is_some() || url.query().is_some() ||
        url.fragment().is_some() || url.path() != "/" {
        return Err("daemon URL must contain only scheme and loopback authority");
    }
    // Numeric loopback only, to prevent DNS/rebinding or hosts-file redirects.
    let host = url.host_str().ok_or("missing daemon address")?;
    let ip = host.trim_start_matches('[').trim_end_matches(']')
        .parse::<IpAddr>().map_err(|_| "numeric loopback IP required")?;
    if !ip.is_loopback() { return Err("daemon URL must target loopback"); }
    Ok(url)
}
async fn fetch_json(state: &AgentState, endpoint: &str) -> Option<Value> {
    let url = state.daemon.join(endpoint).ok()?;
    let mut request = state.client.get(url);
    if let Some(token) = &state.api_token { request = request.bearer_auth(token); }
    let response = request.send().await.ok()?;
    if !response.status().is_success() { return None; }
    response.json().await.ok()
}
fn safe_count(json: &Value, name: &str) -> Option<u64> {
    json.get(name)?.as_u64()
}
async fn inspect_daemon(state: &AgentState) -> DaemonSnapshot {
    let Some(health) = fetch_json(state, "/api/v1/health").await else {
        return DaemonSnapshot::offline();
    };
    if health.get("status").and_then(Value::as_str) != Some("ok") {
        return DaemonSnapshot::offline();
    }
    let version = health.get("version").and_then(Value::as_str)
        .filter(|value| value.len() <= 80)
        .map(ToOwned::to_owned);
    let mut snapshot = DaemonSnapshot { online: true, version, ..DaemonSnapshot::offline() };
    if let Some(stats) = fetch_json(state, "/api/v1/stats").await {
        snapshot.statistics_available = true;
        snapshot.active_tasks = safe_count(&stats, "active_tasks");
        snapshot.completed_tasks = safe_count(&stats, "completed_tasks");
        snapshot.failed_tasks = safe_count(&stats, "failed_tasks");
        snapshot.downloaded_bytes = safe_count(&stats, "total_downloaded_bytes");
        snapshot.uploaded_bytes = safe_count(&stats, "total_uploaded_bytes");
    }
    snapshot
}
async fn info(State(state): State<AgentState>) -> Json<Value> {
    let daemon = inspect_daemon(&state).await;
    Json(json!({
        "agent": "kordra-agent",
        "version": env!("CARGO_PKG_VERSION"),
        "paired": false,
        "remote_control_enabled": false,
        "protocol_version": 1,
        "daemon": daemon.as_json()
    }))
}
async fn local_daemon(State(state): State<AgentState>) -> Json<Value> {
    Json(inspect_daemon(&state).await.as_json())
}
async fn metrics(State(state): State<AgentState>) -> impl IntoResponse {
    let result = inspect_daemon(&state).await;
    let online = u8::from(result.online);
    let stats = u8::from(result.statistics_available);
    let body = format!(
        "# HELP kordra_agent_up Local Kordra-Agent process is running\n\
         # TYPE kordra_agent_up gauge\n\
         kordra_agent_up 1\n\
         # HELP kordra_agent_daemon_online Local Kordra daemon API is healthy\n\
         # TYPE kordra_agent_daemon_online gauge\n\
         kordra_agent_daemon_online {online}\n\
         # HELP kordra_agent_daemon_stats_available Local daemon statistics API is accessible\n\
         # TYPE kordra_agent_daemon_stats_available gauge\n\
         kordra_agent_daemon_stats_available {stats}\n"
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
    let client = Client::builder()
        .redirect(Policy::none()) // Do not follow redirect responses off loopback.
        .timeout(Duration::from_secs(4))
        .build()?;
    let state = AgentState { daemon, client, api_token: env::var("KORDRA_API_TOKEN").ok() };
    let router = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/v1/info", get(info))
        .route("/v1/local-daemon", get(local_daemon))
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
    fn daemon_target_accepts_numeric_loopback_only() {
        for url in ["http://127.0.0.1:8079", "https://127.2.3.4:8079", "http://[::1]:8079"] {
            assert!(validate_daemon_url(url).is_ok(), "{url}");
        }
        for url in [
            "http://example.com", "http://localhost", "http://8.8.8.8",
            "http://0.0.0.0:80", "file:///tmp/socket",
            "http://user:pass@127.0.0.1:8079", "http://127.0.0.1:8079/other",
            "http://127.0.0.1:8079/?next=outside", "http://[::2]:8079"
        ] { assert!(validate_daemon_url(url).is_err(), "{url}"); }
    }
    #[test]
    fn health_metadata_does_not_include_secrets() {
        let snap = DaemonSnapshot {
            online: true, version: Some("0.1.0".to_string()),
            statistics_available: true, active_tasks: Some(2),
            completed_tasks: Some(3), failed_tasks: Some(1),
            downloaded_bytes: Some(1024), uploaded_bytes: Some(256)
        };
        let json = snap.as_json();
        assert_eq!(json["traffic"]["downloaded_bytes"], 1024);
        assert_eq!(json["tasks"]["active"], 2);
        assert!(json.get("api_token").is_none());
    }
}
