# Builder stage
FROM rust:latest AS builder

WORKDIR /app

# Copy Rust project files
COPY src/ ./src/
COPY Cargo.toml .
COPY Cargo.lock .

# Build the application in release mode
RUN cargo build --release

# Final stage
FROM rust:latest

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/read_it_later /app/
COPY templates/ /app/templates/

# Ensure the binary is executable
RUN chmod +x /app/read_it_later
COPY .env /app/.env

# Default command
CMD ["/app/read_it_later"]
