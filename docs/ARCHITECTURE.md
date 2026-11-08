# Kordra-Agent architecture

The independent agent is designed to manage a **local** Kordra headless instance and later communicate securely with authorized remote controllers.

## Implemented (pre-alpha)
- Rust headless agent with a loopback-only HTTP listener.
- `GET /health`, `GET /v1/info`, `GET /v1/local-daemon` and Prometheus `GET /metrics`.
- Checks the Kordra `/api/v1/health` and optional authenticated `/api/v1/stats`, showing daemon version, tasks and aggregate traffic.
- Optional `KORDRA_API_TOKEN` is sent **only** to the configured numeric loopback address. Redirect following is disabled, requests time out, credentials are never returned by the agent.
- `kordra_agent_daemon_online` and `kordra_agent_daemon_stats_available` Prometheus gauges. A healthy daemon without permission to read statistics is reported as online with unavailable stats.
- `protocol_version: 1` is an informational contract for the future authenticated controller protocol, **not** a claim that remote pairing works.

`KORDRA_DAEMON_URL` accepts `http://127.0.0.1:8079` or `http://[::1]:8079` (and numeric loopback HTTPS). Hostnames, paths, queries and fragments are rejected to prevent DNS and redirect surprises.

## Not yet supported
Remote pairing, public listener, remote task administration, agent-initiated transport, certificate management, controlled Kordra installation and daemon supervision. The local read-only diagnostic API is **not** a substitute for authentication. Do not publish it.

Planned remote communication requires one-time pairing, capability-scoped identities, credential rotation/revocation, mutual authentication and a versioned protocol contract. No mandatory central server; LAN/VPN or optional outbound relay.

Tests and builds run only on GitHub-hosted runners; no personal or production servers are used.
