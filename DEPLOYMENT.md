# Deployment Guide (`cdd-engine`)

> This document provides guidance for deploying applications powered by `cdd-engine` across containerized and background daemon environments.

---

## 1. Containerized Deployment (Docker)

`cdd-engine` includes both Alpine (`alpine.Dockerfile`) and Debian (`debian.Dockerfile`) multi-stage container definitions.

```bash
make build_docker
make run_docker
```

---

## 2. Environment Configuration

When deploying a service embedding `cdd-engine` under `systemd`, `launchd`, or Kubernetes, configure runtime behavior via `CDD__*` environment variables:

```ini
Environment="RUST_LOG=info"
Environment="CDD__SERVER_BIND=0.0.0.0:8084"
Environment="CDD__WASM_ASSETS_DIR=/opt/cdd/wasm"
Environment="CDD__WASM_SUPPORT_MANIFEST=/opt/cdd/wasm-support.json"
Environment="CDD__OFFLINE_MODE=true"
```

---

## 3. Process Supervision Best Practices

- Set `stability_threshold_ms` (default `5000` ms) on each `ProcessConfig` so child `cdd-*` MCP servers that remain healthy for at least 5 seconds reset their crash retry counter.
- Set `max_retries` and `restart_delay_ms` to bound restart storms when an external language toolchain binary is misconfigured.
