# Copilot Instructions for embedded-services

## Overview

This is an embedded controller (EC) services workspace — a collection of `no_std` Rust crates implementing hardware-agnostic business logic for embedded controllers. Services glue together MCU HALs (via `embedded-hal` traits), peripheral drivers, and EC subsystem abstractions (sensors, batteries, fans, USB-PD, etc.) using the Embassy async runtime.

## Build, Test, and Lint

Toolchain: Rust 1.88 (`rust-toolchain.toml`), edition 2024. Targets: `x86_64-unknown-linux-gnu` (std/testing) and `thumbv8m.main-none-eabihf` (ARM Cortex-M33).

```shell
# Format
cargo fmt --check

# Lint (all feature combos, both targets)
cargo hack --feature-powerset --mutually-exclusive-features=log,defmt,defmt-timestamp-uptime clippy --locked --target x86_64-unknown-linux-gnu
cargo hack --feature-powerset --mutually-exclusive-features=log,defmt,defmt-timestamp-uptime clippy --locked --target thumbv8m.main-none-eabihf

# Test (workspace, host target only)
cargo test --locked

# Test a single crate
cargo test --locked -p partition-manager

# Test a single test function
cargo test --locked -p partition-manager test_name

# Lint test code
cargo clippy --locked --tests

# Check docs
cargo doc --no-deps -F log --locked
cargo doc --no-deps -F defmt --locked

# Unused dependency check
cargo machete

# Dependency license/advisory/audit checks
cargo deny check --all-features --locked
cargo vet --locked
```

The `examples/` directory contains separate workspaces (excluded from the root workspace). Build/lint them independently:

```shell
# ARM board examples
cd examples/rt685s-evk && cargo clippy --target thumbv8m.main-none-eabihf --locked
# Std examples
cd examples/std && cargo clippy --locked
```

## Architecture

### Service Pattern

Each service crate follows a consistent structure:

1. **Service struct** with a `comms::Endpoint` and domain-specific context/state
2. **`MailboxDelegate` impl** — the `receive()` method handles incoming messages using type-safe downcasting via `message.data.get::<T>()`
3. **Global singleton** — services are stored in `OnceLock<Service>` statics
4. **Async task function** — registers the endpoint, then loops calling a `process()` or `process_next()` method
5. **Spawned via Embassy** — `#[embassy_executor::task]` functions are spawned from main

```rust
// Typical service skeleton
pub struct MyService {
    endpoint: comms::Endpoint,
    // ... domain state
}

impl comms::MailboxDelegate for MyService {
    fn receive(&self, message: &comms::Message) -> Result<(), comms::MailboxDelegateError> {
        if let Some(event) = message.data.get::<MyEvent>() {
            // handle event
        }
        Ok(())
    }
}

static SERVICE: OnceLock<MyService> = OnceLock::new();

pub async fn task() {
    let service = SERVICE.get_or_init(MyService::new);
    comms::register_endpoint(service, &service.endpoint).await.unwrap();
    loop {
        service.process_next().await;
    }
}
```

### Communication (IPC)

Services communicate through `embedded_services::comms` — a type-erased message routing system built on intrusive linked lists (zero allocation):

- **EndpointID**: `Internal(Battery | Thermal | ...)` or `External(Host | Debug | ...)`
- **Messages**: use `&dyn Any` for payload, receivers downcast with `message.data.get::<T>()`
- Services call `embedded_services::init()` before registering endpoints
- Each `EndpointID` has its own static intrusive list of registered endpoints

### Composition

At the top level, an EC is composed by spawning service tasks on an Embassy executor:

```rust
embedded_services::init().await;
spawner.must_spawn(battery_service_task());
spawner.must_spawn(thermal_service_task());
spawner.must_spawn(power_policy_task(config));
```

### Core Utilities (embedded-service crate)

- **`GlobalRawMutex`**: `ThreadModeRawMutex` on ARM bare-metal, `CriticalSectionRawMutex` in tests/std
- **`SyncCell<T>`**: `ThreadModeCell` on ARM, `CriticalSectionCell` elsewhere — interior mutability for embedded
- **`define_static_buffer!`**: macro for creating static buffers with borrow-checked `OwnedRef`/`SharedRef` access
- **`intrusive_list`**: no-alloc linked list using embedded `Node` fields for endpoint routing
- **`Never`**: type alias for `core::convert::Infallible` used in `Result<Never, Error>` for tasks that shouldn't return

## Key Conventions

### `no_std` and Feature Flags

All service crates are `#![no_std]`. Logging is feature-gated with **mutually exclusive** features:

- `defmt` — embedded debug formatting (used on bare-metal targets)
- `log` — standard Rust logging (used on std targets / tests)

These must never be enabled simultaneously in production. Use the unified macros from `embedded_services::fmt` (`trace!`, `debug!`, `info!`, `warn!`, `error!`) which dispatch to the correct backend.

### Error Handling

- Custom `enum` error types per module — no `thiserror` (it requires std)
- All error enums derive `Debug, Clone, Copy, PartialEq, Eq`
- Conditional defmt support: `#[cfg_attr(feature = "defmt", derive(defmt::Format))]`
- Result type aliases per module (e.g., `pub type BatteryResponse = Result<ContextResponse, ContextError>`)

### Clippy Configuration

The workspace enforces strict clippy lints (in root `Cargo.toml`). Key denials:

- `unwrap_used`, `expect_used`, `panic`, `unreachable`, `unimplemented`, `todo` — no panicking in production code
- `indexing_slicing` — use `.get()` instead of `[]`
- Tests can relax these with `#[allow(clippy::panic)]`, `#[allow(clippy::unwrap_used)]`

### Dependencies

- Workspace dependencies are centralized in root `Cargo.toml` under `[workspace.dependencies]`; member crates use `dep.workspace = true`
- Git dependencies from the OpenDevicePartnership GitHub org (embassy-imxrt, embedded-usb-pd, tps6699x, etc.)
- Feature-gated optional deps (`log`, `defmt`) should be listed in `[package.metadata.cargo-machete] ignored` to avoid false positives
- Supply chain security enforced via `cargo-vet` (config in `supply-chain/`) and `cargo-deny` (config in `deny.toml`)

### Testing

- Async tests use `embassy_futures::block_on(async { ... })`
- Dev-dependencies enable `std` features: `embassy-sync/std`, `embassy-time/std`, `critical-section/std`
- `tokio` with `rt`, `macros`, `time` features for integration tests
- Tests are organized in `#[cfg(test)]` modules or dedicated `test/` subdirectories

### Formatting

Max line width is 120 characters (`rustfmt.toml`).
