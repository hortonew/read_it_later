# Builder stage
FROM rust:latest AS builder

WORKDIR /app

# Step 1: Copy dependency files to pre-cache dependencies
COPY Cargo.toml .
COPY Cargo.lock .

# Step 2: Create a dummy main.rs to allow dependency installation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Step 3: Pre-cache dependencies by building the dummy project
RUN cargo build --release

# Step 4: Copy the actual source code
COPY src/ ./src/

# Step 5: Rebuild the application with the real code
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

WORKDIR /app

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    libssl3 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Copy only the binary and required assets from the builder stage
COPY --from=builder /app/target/release/read_it_later /app/
COPY templates/ /app/templates/
COPY .env /app/.env

# Ensure the binary is executable
RUN chmod +x /app/read_it_later

# Set the default command
CMD ["/app/read_it_later"]
