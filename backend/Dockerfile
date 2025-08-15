# ---- Build Stage ----
FROM rust:latest as build
WORKDIR /app
COPY . .
RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim
LABEL maintainer="Your Name <your@email.com>"
LABEL org.opencontainers.image.source="https://github.com/your/repo"
WORKDIR /app
COPY --from=build /app/target/release/backend /app/backend
# Create a non-root user and use it
RUN mkdir -p /var/log/url_shortener && chmod 755 /var/log/url_shortener
RUN useradd -m appuser && chown -R appuser /app
USER appuser
# Expose the backend port
EXPOSE 8080
# Set environment variables (override with Docker/K8s secrets in prod)
ENV APP_ENV=production
ENV RUST_LOG=info
CMD ["/app/backend"]
