# Builder and runtime are both Debian 12 (bookworm) so the binary's glibc matches. The
# unsuffixed cargo-chef tags are trixie (Debian 13), which would not run on cc-debian12.
FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99.0-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Dependencies build in their own layer, cached until Cargo.toml/Cargo.lock change.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked --bin gr-logistics

# No shell, no package manager, runs as non-root. Migrations are embedded in the binary.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/gr-logistics /usr/local/bin/gr-logistics
ENV NODE_ENV=production
EXPOSE 3000
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD ["gr-logistics", "healthcheck"]
ENTRYPOINT ["gr-logistics"]
CMD ["serve"]
