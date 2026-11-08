# Kordra-Agent architecture

The independent agent is designed to manage a **local** Kordra headless instance and later communicate securely with authorized remote controllers.

## Implemented (pre-alpha)
- Rust headless binary with loopback-only HTTP listener.
- Local `GET /health`, `GET /v1/info`, and Prometheus `GET /metrics`.
- Local Kordra daemon health probe at configurable `KORDRA_DAEMON_URL`, restricted to loopback HTTP(S) URLs.
- Optional `KORDRA_API_TOKEN` used only for requests to the local daemon.
- `kordra_agent_daemon_online` gauge and `daemon_online` in the info response.

## Not yet supported
Remote pairing, public listener, remote task administration, agent-initiated transport, certificate management, controlled Kordra installation and daemon supervision. The local read-only diagnostic API is **not** a substitute for authentication. Do not publish it.

Planned remote communication requires one-time pairing, capability-scoped identities, credential rotation/revocation, mutual authentication and a versioned protocol contract. No mandatory central server; LAN/VPN or optional outbound relay.

Tests and builds run only on GitHub-hosted runners; no personal or production servers are used.
