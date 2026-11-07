# Agent architecture

The agent is a separate headless Rust binary for a device running Kordra.

Planned workflow:
1. Discover a local Kordra daemon over loopback.
2. User authorizes one-time pairing through the controller.
3. Exchange short-lived device identity credentials and pin the controller identity.
4. Establish authenticated encrypted communications over LAN/VPN or outbound agent-initiated transport.
5. Expose **capability-scoped** calls for health, progress, task control and metadata.
6. Rotate and revoke device credentials; report connectivity and error metrics.

The controller does not require a vendor-hosted server. Nodes behind NAT need LAN/VPN connectivity or an optional authenticated relay.

Agent **does not** duplicate download engines or Kordra's database. Its package is independently versioned; protocol compatibility is explicit. Until the pairing flow, authorization and permission checks exist, the service is **strictly loopback only**, and remote execution is intentionally unavailable.
