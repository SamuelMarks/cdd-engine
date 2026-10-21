# Stage 1: Build & Verify cdd-engine
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev gcc pkgconfig

WORKDIR /usr/src/cdd-engine
COPY . .

RUN cargo build --release && cargo test --release

FROM alpine:latest
WORKDIR /usr/src/cdd-engine
COPY --from=builder /usr/src/cdd-engine/Cargo.toml .
CMD ["echo", "cdd-engine verified"]
