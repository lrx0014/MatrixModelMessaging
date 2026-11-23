# Build on the builder's native CPU; only C/Rust target code is cross compiled.
FROM --platform=$BUILDPLATFORM node:26.13.1-bookworm-slim@sha256:5df79989ed685e09f0a98019db4cebfb22fb9df1a53c0161664a011f7ff6b0f1 AS contracts
WORKDIR /source
COPY .node-version .node-version
COPY api api
COPY web web
RUN cd web && npm ci --ignore-scripts && npm run generate:check && npm run typecheck && touch /contracts-checked

FROM --platform=$BUILDPLATFORM rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS build
ARG BUILDARCH
ARG TARGETARCH
RUN apt-get update && apt-get install --yes --no-install-recommends cmake && \
    if [ "$BUILDARCH" != "$TARGETARCH" ]; then \
      case "$TARGETARCH" in \
        amd64) apt-get install --yes gcc-x86-64-linux-gnu g++-x86-64-linux-gnu ;; \
        arm64) apt-get install --yes gcc-aarch64-linux-gnu g++-aarch64-linux-gnu ;; \
        *) exit 1 ;; \
      esac; \
    fi && rm -rf /var/lib/apt/lists/*
ENV CARGO_BUILD_JOBS=1 \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
WORKDIR /source
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY .sqlx .sqlx
COPY crates crates
COPY xtask xtask
COPY --from=contracts /contracts-checked /contracts-checked
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/source/target,sharing=locked \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu ;; \
      arm64) triple=aarch64-unknown-linux-gnu ;; \
      *) exit 1 ;; \
    esac && rustup target add "$triple" && \
    cargo build --release --locked --workspace --target "$triple" && \
    mkdir /out && cp target/"$triple"/release/m3-control target/"$triple"/release/m3-agent target/"$triple"/release/m3 /out/

# Execute the identical CI entry point inside a native Linux builder.
FROM build AS verification
COPY --from=contracts /usr/local/bin/node /usr/local/bin/node
COPY --from=contracts /usr/local/lib/node_modules /usr/local/lib/node_modules
COPY --from=contracts /source/web /source/web
COPY .node-version .node-version
COPY api api
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm && rustup component add rustfmt clippy
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/source/target,sharing=locked \
    cargo xtask verify

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime
RUN apt-get update && apt-get install --yes --no-install-recommends ca-certificates libgcc-s1 && rm -rf /var/lib/apt/lists/*
USER 65532:65532
WORKDIR /tmp

FROM runtime AS control
COPY --from=build /out/m3-control /out/m3 /usr/local/bin/
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=6s CMD ["/usr/local/bin/m3", "health"]
ENTRYPOINT ["/usr/local/bin/m3-control"]
CMD ["--listen", "0.0.0.0:8080"]

FROM runtime AS agent
COPY --from=build /out/m3-agent /usr/local/bin/m3-agent
ENTRYPOINT ["/usr/local/bin/m3-agent"]
CMD ["--help"]
