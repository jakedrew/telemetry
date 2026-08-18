FROM node:22-slim AS web
WORKDIR /web
COPY web/telemetry ./
RUN npm ci && npm run build

FROM rust:1 AS rust
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=caddy:2.11.4 /usr/bin/caddy /usr/bin/caddy
COPY --from=rust /app/target/release/telemetry /usr/local/bin/telemetry
COPY --from=web /web/dist /srv
COPY Caddyfile /etc/caddy/Caddyfile

EXPOSE 80 443
CMD /usr/local/bin/telemetry & caddy run --config /etc/caddy/Caddyfile