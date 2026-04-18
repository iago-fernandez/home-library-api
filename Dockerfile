FROM rust:1.80-slim AS builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/home_library_api*

COPY src ./src
COPY .env ./
RUN cargo build --release

FROM debian:bookworm-slim

RUN groupadd -r appgroup && useradd -r -g appgroup appuser

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/home-library-api .
COPY --from=builder /usr/src/app/.env .

RUN chown -R appuser:appgroup /app
USER appuser

EXPOSE 3000

CMD ["./home-library-api"]