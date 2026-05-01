# Rūsternetes local node image used by rusternetes-cluster-manager.
# It builds the existing all-in-one Rusternetes binary and packages it as
# /usr/local/bin/rusternetesd so the top-level `rusternetes` command can be the CLI.
FROM rust:1.81-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p rusternetes --features sqlite
RUN mkdir -p /out/console && \
    if [ -d console/dist ]; then cp -R console/dist/. /out/console/; \
    elif [ -d console/build ]; then cp -R console/build/. /out/console/; \
    else printf '%s\n' '<!doctype html><html><body><h1>Rūsternetes Console</h1><p>Build console assets to enable the full desktop/web console.</p></body></html>' > /out/console/index.html; fi

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl iproute2 iptables procps && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/rusternetes /usr/local/bin/rusternetesd
COPY --from=builder /out/console /opt/rusternetes/console
RUN mkdir -p /var/lib/rusternetes/storage /var/lib/rusternetes/volumes /var/log/rusternetes
EXPOSE 6443
ENTRYPOINT ["/usr/local/bin/rusternetesd"]
