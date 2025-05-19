# --- Build Stage ---
    FROM rust:1.87 as builder

    WORKDIR /usr/src/app
    COPY . .
    
    # Compile the app in release mode
    RUN cargo build --release
    
    # --- Runtime Stage ---
    FROM debian:bookworm-slim
    
    # Install necessary dependencies (if any)
    RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
    
    # Copy the compiled binary from the builder
    COPY --from=builder /usr/src/app/target/release/rust-image-upload-processor /usr/local/bin/rust-image-upload-processor
    
    # Set the startup command
    CMD ["rust-image-upload-processor"]