# Stage 1: Build & Verify cdd-engine
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config gcc build-essential && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/cdd-engine
COPY . .

RUN cargo build --release && cargo test --release

FROM debian:bookworm-slim
WORKDIR /usr/src/cdd-engine
COPY --from=builder /usr/src/cdd-engine/Cargo.toml .
CMD ["echo", "cdd-engine verified"]
