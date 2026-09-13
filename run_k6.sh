#!/usr/bin/env bash
set -e

NETWORK="mini-warehouse-wms_default"
SCRIPT="${1:-k6/wms_api_test.js}"
BASE_URL="${BASE_URL:-http://api-gateway:8080}"

echo "Running k6 ($SCRIPT) against $BASE_URL on network $NETWORK..."
docker run --rm -i \
    --network "$NETWORK" \
    -e BASE_URL="$BASE_URL" \
    grafana/k6 run - < "$SCRIPT"
