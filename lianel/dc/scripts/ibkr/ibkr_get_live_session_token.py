#!/usr/bin/env python3
"""
Call IBKR POST /oauth/live_session_token and print response.
Uses env: IBKR_OAUTH_* and PEM paths. Exit 0 on 200 + LST, non-zero otherwise.
Run on server where .env and PEMs exist, or set env and paths.
"""
import base64
import hashlib
import json
import os
import random
import string
import sys
import time
from urllib.parse import quote, quote_plus

try:
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import dh, padding, rsa
    from cryptography.hazmat.backends import default_backend
except ImportError:
    print("ERROR: pip install cryptography", file=sys.stderr)
    sys.exit(2)


def rfc3986_quote(s: str) -> str:
    return quote(s, safe="-._~")


def doc_style_param_string(params: dict) -> str:
    """Match IBKR doc: unencoded key=value&... then we quote() the whole string for base."""
    return "&".join(f"{k}={v}" for k, v in sorted(params.items()))


def main() -> int:
    consumer_key = os.environ.get("IBKR_OAUTH_CONSUMER_KEY", "").strip()
    access_token = os.environ.get("IBKR_OAUTH_ACCESS_TOKEN", "").strip()
    access_token_secret_b64 = os.environ.get("IBKR_OAUTH_ACCESS_TOKEN_SECRET", "").strip()
    realm = os.environ.get("IBKR_OAUTH_REALM", "test_realm").strip() or "test_realm"
    base_url = (os.environ.get("IBKR_API_BASE_URL", "https://api.ibkr.com/v1/api") or "https://api.ibkr.com/v1/api").rstrip("/")
    dh_path = os.environ.get("IBKR_OAUTH_DH_PARAM_PATH", "")
    enc_path = os.environ.get("IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH", "")
    sig_path = os.environ.get("IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH", "")

    for name, val in [
        ("IBKR_OAUTH_CONSUMER_KEY", consumer_key),
        ("IBKR_OAUTH_ACCESS_TOKEN", access_token),
        ("IBKR_OAUTH_ACCESS_TOKEN_SECRET", access_token_secret_b64),
        ("IBKR_OAUTH_DH_PARAM_PATH", dh_path),
        ("IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH", enc_path),
        ("IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH", sig_path),
    ]:
        if not val or not os.path.isfile(val) if "PATH" in name else not val:
            if "PATH" in name and not os.path.isfile(val):
                # Avoid leaking filesystem paths (and any secrets embedded in them) into logs.
                print(f"ERROR: missing or not a file: {name}", file=sys.stderr)
            elif "PATH" not in name and not val:
                # Avoid echoing sensitive env values into logs.
                print(f"ERROR: missing env {name}", file=sys.stderr)
            sys.exit(2)

    # Load DH parameters (PEM: DH PARAMETERS or PKCS#8 RSA as fallback for old format)
    with open(dh_path, "rb") as f:
        dh_pem = f.read()
    try:
        if b"DH PARAMETERS" in dh_pem:
            params = serialization.load_pem_parameters(dh_pem)
            pn = params.parameter_numbers()
            dh_p, dh_g = pn.p, pn.g
        else:
            key = serialization.load_pem_private_key(dh_pem, password=None, backend=default_backend())
            if isinstance(key, rsa.RSAPrivateKey):
                nums = key.private_numbers()
                dh_p, dh_g = nums.public_numbers.n, nums.public_numbers.e
            else:
                print("ERROR: DH file must be DH PARAMETERS or RSA PEM", file=sys.stderr)
                sys.exit(2)
    except Exception as e:
        print(f"ERROR: loading DH params: {e}", file=sys.stderr)
        sys.exit(2)

    with open(enc_path, "rb") as f:
        enc_key = serialization.load_pem_private_key(f.read(), password=None, backend=default_backend())
    with open(sig_path, "rb") as f:
        sig_key = serialization.load_pem_private_key(f.read(), password=None, backend=default_backend())

    encrypted_secret = base64.b64decode(access_token_secret_b64)
    prepend_bytes = enc_key.decrypt(encrypted_secret, padding.PKCS1v15())
    prepend_hex = prepend_bytes.hex()

    dh_random = int.from_bytes(os.urandom(32), "big")
    dh_challenge = pow(dh_g, dh_random, dh_p)
    dh_challenge_hex = format(dh_challenge, "x")
    if len(dh_challenge_hex) % 2:
        dh_challenge_hex = "0" + dh_challenge_hex

    lst_url = f"{base_url}/oauth/live_session_token"
    timestamp = int(time.time())
    nonce = "".join(random.choices(string.ascii_letters + string.digits, k=32))

    # Build params for base string (no oauth_signature or realm yet).
    oauth_params = {
        "diffie_hellman_challenge": dh_challenge_hex,
        "oauth_consumer_key": consumer_key,
        "oauth_nonce": nonce,
        "oauth_signature_method": "RSA-SHA256",
        "oauth_timestamp": str(timestamp),
        "oauth_token": access_token,
    }
    # Match IBKR doc: base string = prepend + method + "&" + quote_plus(url) + "&" + quote(params_string)
    params_string = doc_style_param_string(oauth_params)
    base_string = f"{prepend_hex}POST&{quote_plus(lst_url)}&{quote(params_string)}"

    signature = sig_key.sign(base_string.encode("utf-8"), padding.PKCS1v15(), hashes.SHA256())
    sig_b64 = base64.b64encode(signature).decode("ascii")
    oauth_params["oauth_signature"] = quote_plus(sig_b64)  # doc uses quote_plus for signature
    oauth_params["realm"] = realm

    auth_header = "OAuth " + ", ".join(f'{k}="{v.replace(chr(92), chr(92)+chr(92)).replace(chr(34), chr(92)+chr(34))}"' for k, v in sorted(oauth_params.items()))

    try:
        import urllib.request
        req = urllib.request.Request(lst_url, data=b"", method="POST")
        req.add_header("Authorization", auth_header)
        req.add_header("User-Agent", "lianel-stock-service/1.0")
        req.add_header("Accept", "*/*")
        req.add_header("Content-Length", "0")
        with urllib.request.urlopen(req, timeout=30) as resp:
            body = resp.read().decode("utf-8")
            code = resp.getcode()
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8")
        code = e.code
    except Exception as e:
        print(f"ERROR: request failed: {e}", file=sys.stderr)
        sys.exit(3)

    print(f"HTTP {code}")
    print(body)
    if code == 200:
        try:
            data = json.loads(body)
            if "live_session_token_signature" in data and "diffie_hellman_response" in data:
                print("\n*** GREEN: Live session token response received from IBKR ***", file=sys.stderr)
                return 0
        except json.JSONDecodeError:
            pass
        print("\n*** Unexpected 200 body ***", file=sys.stderr)
        return 1
    print(f"\n*** IBKR returned {code}; fix config and retry ***", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
