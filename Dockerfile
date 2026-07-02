# Build stage
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/src/acp-agent

COPY . .

RUN cargo build --release

# Production stage
FROM alpine:latest

WORKDIR /app

COPY --from=builder /usr/src/acp-agent/target/release/acp-agent-wrapper /app/acp-agent-wrapper

ENTRYPOINT ["/app/acp-agent-wrapper"]
