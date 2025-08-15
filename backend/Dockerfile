# syntax=docker/dockerfile:1
FROM rust:1.82
WORKDIR /app
COPY . .
RUN cargo build --release
COPY migrations ./migrations
COPY .env .env
EXPOSE 8080
CMD ["/app/target/release/backend"]
