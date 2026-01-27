FROM rust:1-alpine3.22 AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

RUN rustup toolchain install nightly && \
    rustup default nightly && \
    rustup target add x86_64-unknown-linux-musl

# Copy all source code
COPY . .

# remove debug symbols
ENV RUSTFLAGS="-C link-arg=-s"

# Build the binary
RUN cargo build --release --target x86_64-unknown-linux-musl --bin steel && \
    strip --strip-all target/x86_64-unknown-linux-musl/release/steel && \
    rm -rf target/x86_64-unknown-linux-musl/release/deps target/x86_64-unknown-linux-musl/release/build target/x86_64-unknown-linux-musl/release/incremental

# (scratch = empty image)
FROM scratch

# Copy only the binary and required assets
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/steel /steel
COPY --from=builder /app/package-content/favicon.png /config/favicon.png
COPY --from=builder /app/package-content/steel_config.json5 /config/steel_config.json5

EXPOSE 25565

CMD ["/steel"]