#!/bin/sh
set -e

if [ -z "$HYDRO_BINARY" ]; then
    echo "ERROR: HYDRO_BINARY environment variable not set" >&2
    exit 1
fi

exec "/usr/local/bin/$HYDRO_BINARY" "$@"
