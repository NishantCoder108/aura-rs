# --- Stage 1: Fast Build Cache ---
    # Start a new build stage named 'builder' using the 'rust:1.78-slim' base image
    FROM rust:1.78-slim AS builder
    # Set the working directory to '/app' in the builder stage
    WORKDIR /app
    
    # Install system dependencies required for compilation toolchains in the builder stage
    # This includes 'pkg-config' which is used to locate the needed header files and libraries for compilation,
    # and 'libssl-dev' which includes the OpenSSL development libraries and header files.
    RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
    
    # Copy the entire project source tree from the host machine to the builder stage
    COPY . .
    
    # Compile the project binary in release mode (optimized for performance) in the builder stage
    RUN cargo build --release

# --- Stage 2: Minimal Distroless Runtime ---
    # Start a new build stage named 'builder' using the 'debian:bookworm-slim' base image
    FROM debian:bookworm-slim
    # Set the working directory to '/app' in the runtime stage
    WORKDIR /app

    # Install OpenSSL certificates so the binary can securely talk to external database APIs in the runtime stage
    RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

    # Copy the compiled binary from the builder stage to the runtime stage
    # IMPORTANT: Change 'your_binary_name' to the exact name specified in your Cargo.toml file under [package] name
    COPY --from=builder /app/target/release/urlvibe-rs ./rust_engine

    # Expose the standard Hugging Face proxy entry port
    EXPOSE 7860
    # Set the environment variable 'PORT' to '7860'
    ENV PORT=7860

    # Execute the light binary in the runtime stage
    CMD ["./rust_engine"]