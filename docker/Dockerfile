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
COPY cuttle-rust ./cuttle-rust
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
