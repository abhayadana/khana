#!/bin/sh
# Compatibility entry point. The package now lives in dist/.
set -eu
cd "$(dirname "$0")"
exec ./scripts/package-khana.sh
