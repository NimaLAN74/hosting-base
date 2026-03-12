#!/usr/bin/env bash
# Obtain IBKR Gateway session cookie and write to a file so stock-service can use it
# (backend reads IBKR_GATEWAY_SESSION_COOKIE_FILE on each request).
# Run from server: ./scripts/refresh-gateway-cookie.sh
# Requires: .env with IBKR_USERNAME, IBKR_PASSWORD; docker image ibkr-gateway-login:latest; optional COOKIE_FILE env.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
COOKIE_FILE="${IBKR_GATEWAY_COOKIE_FILE:-$DC_DIR/stock-service-cookie/cookie.txt}"
GATEWAY_URL="${IBKR_GATEWAY_URL:-https://www.lianel.se/ibkr-gateway/}"

mkdir -p "$(dirname "$COOKIE_FILE")"
if [ ! -f "$DC_DIR/.env" ]; then
  echo "Missing $DC_DIR/.env" >&2
  exit 1
fi

echo "Running gateway-login (GATEWAY_URL=$GATEWAY_URL)..." >&2
COOKIE=$(docker run --rm --network host --shm-size=1g \
  --env-file "$DC_DIR/.env" \
  -e GATEWAY_URL="$GATEWAY_URL" \
  -e NODE_TLS_REJECT_UNAUTHORIZED=0 \
  ibkr-gateway-login:latest 2>/dev/null | tail -1)

if [ -n "$COOKIE" ] && [ "${#COOKIE}" -gt 5 ]; then
  echo "$COOKIE" > "$COOKIE_FILE"
  echo "Cookie written to $COOKIE_FILE (from gateway-login)" >&2
  exit 0
fi

# Fallback: extract from IBeam Chrome profile if IBeam has logged in
if docker ps --format '{{.Names}}' | grep -q '^ibkr-gateway$'; then
  echo "Trying to extract cookie from IBeam Chrome profile..." >&2
  TMP_COOKIES="/tmp/ibkr-cookies-$$.db"
  if docker cp ibkr-gateway:/tmp/ibeam-chrome-default/Default/Cookies "$TMP_COOKIES" 2>/dev/null; then
    if command -v python3 >/dev/null 2>&1; then
      COOKIE=$(python3 "$SCRIPT_DIR/extract-ibeam-cookie.py" "$TMP_COOKIES" 2>/dev/null)
      rm -f "$TMP_COOKIES"
      if [ -n "$COOKIE" ] && [ "${#COOKIE}" -gt 5 ]; then
        echo "$COOKIE" > "$COOKIE_FILE"
        echo "Cookie written to $COOKIE_FILE (from IBeam profile)" >&2
        exit 0
      fi
    fi
  fi
  rm -f "$TMP_COOKIES"
fi

echo "Gateway login failed and IBeam cookie extraction failed or not available" >&2
exit 1
