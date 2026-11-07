# Kordra-Agent

Separate lightweight device agent for [Kordra](https://github.com/KordraProject/Kordra).

**Current state: pre-alpha.** Only loopback diagnostic endpoints are available. No remote code execution, download dispatch, credential enrollment or remote management until authenticated pairing has been implemented.

## Structure
- `apps/kordra-agent`: Rust headless service.
- `docs/ARCHITECTURE.md`: planned identity, pairing and communication design.
- `.github/workflows`: GitHub-hosted Windows/Linux checks and headless artifacts.

Kordra-Agent shares a **versioned API protocol** with Kordra rather than copying its download engines.

Builds and tests run exclusively through GitHub-hosted runners: no external servers, local machines, VPS or self-hosted runners.
