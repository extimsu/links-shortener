# ---- Build Stage ----
FROM messense/cargo-zigbuild:latest as builder

WORKDIR /app

# 1. Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm src/main.rs

# 2. Copy source and build
COPY src/ ./src/

RUN cargo zigbuild --release --bin backend --target aarch64-unknown-linux-musl

# Runtime stage
FROM scratch
LABEL maintainer="Alexander Extim <it@extim.su>"
LABEL org.opencontainers.image.source="https://github.com/extimsu/backend"
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/backend /usr/local/bin/backend
ENV APP_ENV=production
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/backend"]
