# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo check                        # Quick syntax/type check
cargo nextest run                  # Run tests (preferred)
cargo test --all -- --show-output  # Run tests with output (used in CI)
cargo nextest run <test_name>      # Run a single test
cargo +nightly fmt                 # Format code (requires nightly toolchain)
cargo fmt --all -- --check         # Check formatting without modifying
cargo build --all                  # Build the project
```

## Architecture

govee2mqtt bridges Govee smart home devices to Home Assistant via MQTT. It's an async Rust application using Tokio.

### Protocol Priority

The app tries APIs in this order, preferring the lowest-latency option:
1. **LAN API** (`src/lan_api.rs`) — direct UDP, no internet required
2. **Undocumented AWS IoT API** (`src/undoc_api.rs`) — real-time MQTT via AWS IoT, requires Govee credentials
3. **Platform API** (`src/platform_api.rs`) — official Govee developer API, requires API key
4. **REST API** (`src/rest_api.rs`) — legacy HTTP API v2

### Core Service Layer (`src/service/`)

- `state.rs` — Central `StateHandle = Arc<State>` managing all devices, client references, and caching
- `device.rs` — `Device` struct wrapping LAN, platform, and IoT representations of the same physical device
- `hass.rs` — MQTT integration: publishes discovery messages and handles commands from Home Assistant
- `iot.rs` — AWS IoT MQTT client with certificate management
- `coordinator.rs` — Serializes device control across APIs using per-device semaphores
- `quirks.rs` — Device-specific behavior overrides for model-specific capabilities

### Home Assistant MQTT Entities (`src/hass_mqtt/`)

Each device can expose multiple entity types (light, switch, climate, humidifier, cover, sensor, etc.). `work_mode.rs` and `enumerator.rs` handle dynamic entity creation based on device capabilities.

### Testing

Uses `k9` for snapshot testing. Snapshot files live in `src/__k9_snapshots__/`. Test data fixtures are in `test-data/`.

### Key Patterns

- **State access**: Always through `StateHandle` (an `Arc<State>`)
- **API clients**: Each API has a cloneable, thread-safe client type stored in `State`
- **Device control**: Goes through `coordinator.rs` to serialize concurrent commands
- **Caching**: SQLite-backed (`src/cache.rs`) with soft/hard TTL, used for expensive API calls
- **Configuration**: Via environment variables (with `*_FILE` variants for Docker secrets); see `docs/CONFIG.md`

### Formatting

Code uses 4-space indentation, `imports_granularity = "Module"`, Rust 2021 edition. Formatting requires nightly: `cargo +nightly fmt`.
