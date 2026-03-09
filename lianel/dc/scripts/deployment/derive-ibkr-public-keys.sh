#!/bin/bash
# Derive public key PEMs from IBKR private key PEMs for upload to IBKR OAuth portal.
# Input: private_signature.pem, private_encryption.pem (PKCS#8 RSA).
# Output: public_signing_key.pem, public_encryption_key.pem (standard PEM).
#
# Usage:
#   ./derive-ibkr-public-keys.sh [dir]
#   Default dir: ./stock-service-ibkr (relative to script) or $IBKR_KEYS_DIR
#   Or set: PRIVATE_SIG_PEM=path PRIVATE_ENC_PEM=path OUT_DIR=path
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYS_DIR="${1:-${IBKR_KEYS_DIR:-$SCRIPT_DIR/keys}}"
OUT_DIR="${OUT_DIR:-$KEYS_DIR}"

PRIVATE_SIG="${PRIVATE_SIG_PEM:-$KEYS_DIR/private_signature.pem}"
PRIVATE_ENC="${PRIVATE_ENC_PEM:-$KEYS_DIR/private_encryption.pem}"
PUBLIC_SIG_OUT="${OUT_DIR}/public_signing_key.pem"
PUBLIC_ENC_OUT="${OUT_DIR}/public_encryption_key.pem"

for f in "$PRIVATE_SIG" "$PRIVATE_ENC"; do
  if [[ ! -f "$f" ]]; then
    echo "Missing: $f" >&2
    echo "Usage: Put private_signature.pem and private_encryption.pem in $KEYS_DIR, or set PRIVATE_SIG_PEM and PRIVATE_ENC_PEM." >&2
    exit 1
  fi
done

mkdir -p "$OUT_DIR"

# Derive public key PEM (SubjectPublicKeyInfo) from private key
openssl pkey -in "$PRIVATE_SIG" -pubout -out "$PUBLIC_SIG_OUT"
openssl pkey -in "$PRIVATE_ENC" -pubout -out "$PUBLIC_ENC_OUT"

echo "Created (upload these to IBKR OAuth portal):"
echo "  $PUBLIC_SIG_OUT"
echo "  $PUBLIC_ENC_OUT"
