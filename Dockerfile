# Build stage
FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev cmake make perl ca-certificates

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked && rm -rf src

# Copy actual source and rebuild
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

# Runtime stage - minimal image
FROM scratch

# Copy CA certificates for HTTPS
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy binary
COPY --from=builder /app/target/release/metaseek /metaseek

EXPOSE 8888

ENTRYPOINT ["/metaseek"]
