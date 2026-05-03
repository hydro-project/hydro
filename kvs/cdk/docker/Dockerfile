FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy pre-built Hydro binaries (built by build.sh before cdk deploy)
COPY bin/ /usr/local/bin/hydro/

ENV HYDRO_BINARY=""

ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/hydro/${HYDRO_BINARY}"]
