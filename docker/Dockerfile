FROM node:22.21 AS js-deps

WORKDIR /cuttle
ENV NPM_CONFIG_LOGLEVEL=notice
COPY package*.json ./
RUN npm ci --legacy-peer-deps

FROM js-deps AS js-build

WORKDIR /cuttle
COPY . .
RUN npm run build

FROM rust:1.93.0-bookworm AS rust-build

WORKDIR /rust-src
# Copy only Cargo manifests first so dependency compilation can be cached
# unless dependencies change.
COPY cuttle-rust/Cargo.toml cuttle-rust/Cargo.lock cuttle-rust/rust-toolchain.toml ./cuttle-rust/
COPY cuttle-rust/crates/cutthroat_engine/Cargo.toml ./cuttle-rust/crates/cutthroat_engine/Cargo.toml
COPY cuttle-rust/crates/cutthroat_server/Cargo.toml ./cuttle-rust/crates/cutthroat_server/Cargo.toml
# Create minimal placeholder sources so cargo can resolve and build deps
# before the full Rust source tree is copied.
RUN mkdir -p ./cuttle-rust/crates/cutthroat_engine/src ./cuttle-rust/crates/cutthroat_server/src
RUN printf '%s\n' 'pub fn _docker_cache_sentinel() {}' > ./cuttle-rust/crates/cutthroat_engine/src/lib.rs
RUN printf '%s\n' 'fn main() {}' > ./cuttle-rust/crates/cutthroat_server/src/main.rs
# Warm dependency/build cache.
RUN cargo build --manifest-path ./cuttle-rust/Cargo.toml -p cutthroat_server --release
# Copy real Rust sources and do the actual release build.
COPY cuttle-rust ./cuttle-rust
# Force source mtimes newer than the cached placeholder build so cargo rebuilds
# the real server binary (and not the sentinel `fn main(){}` artifact).
RUN find ./cuttle-rust/crates -type f -name '*.rs' -exec touch {} +
RUN cargo build --manifest-path ./cuttle-rust/Cargo.toml -p cutthroat_server --release

FROM node:22.21

WORKDIR /cuttle
ENV NPM_CONFIG_LOGLEVEL=notice

COPY --from=js-build /cuttle /cuttle
COPY --from=rust-build /rust-src/cuttle-rust/target/release/cutthroat_server /usr/local/bin/cutthroat_server
COPY docker/start-services.sh /usr/local/bin/start-services.sh

EXPOSE 8080
EXPOSE 1337

CMD ["/usr/local/bin/start-services.sh"]
