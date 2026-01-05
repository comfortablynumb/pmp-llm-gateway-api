# Build stage
FROM rust:1.92-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/pmp-llm-gateway /app/pmp-llm-gateway

# Copy config files if needed
COPY config ./config

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV APP__SERVER__HOST=0.0.0.0
ENV APP__SERVER__PORT=8080

# Run the binary with serve command
CMD ["./pmp-llm-gateway", "serve"]
