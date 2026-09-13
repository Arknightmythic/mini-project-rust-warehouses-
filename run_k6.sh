#!/usr/bin/env bash
set -e

NETWORK="mini-warehouse-wms_default"
BASE_URL="${BASE_URL:-http://app:8080}"

echo "Running k6 load test against $BASE_URL on network $NETWORK..."
docker run --rm -i \
    --network "$NETWORK" \
    -e BASE_URL="$BASE_URL" \
    grafana/k6 run - < k6/wms_api_test.js
