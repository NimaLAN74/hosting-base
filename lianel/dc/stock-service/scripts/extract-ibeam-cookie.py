#!/usr/bin/env python3
"""
Read Chrome Cookies SQLite (from IBeam container) and print the 'api' cookie value.
Linux Chrome uses hardcoded key (peanuts/saltysalt). Usage:
  docker cp ibkr-gateway:/tmp/ibeam-chrome-default/Default/Cookies /tmp/cookies.db
  python3 extract-ibeam-cookie.py /tmp/cookies.db
"""
import os
import sys
import sqlite3

# Linux Chrome cookie decryption (hardcoded key)
def decrypt_linux_chrome(encrypted: bytes) -> str:
    if not encrypted or not encrypted.startswith(b'v10') and not encrypted.startswith(b'v11'):
        return encrypted.decode('utf-8', errors='replace') if isinstance(encrypted, bytes) else str(encrypted)
    try:
        from Crypto.Cipher import AES
        from Crypto.Protocol.KDF import PBKDF2
        key = PBKDF2(b'peanuts', b'saltysalt', dkLen=16, count=1)
        cipher = AES.new(key, AES.MODE_CBC, iv=b' ' * 16)
        raw = cipher.decrypt(encrypted[3:])  # strip v10/v11
        return raw.rstrip(b' ').decode('utf-8', errors='replace')
    except Exception:
        return encrypted.decode('utf-8', errors='replace') if isinstance(encrypted, bytes) else str(encrypted)

def main():
    if len(sys.argv) < 2:
        print("Usage: extract-ibeam-cookie.py <path-to-Cookies-sqlite>", file=sys.stderr)
        sys.exit(1)
    path = sys.argv[1]
    if not os.path.exists(path):
        print(f"File not found: {path}", file=sys.stderr)
        sys.exit(1)
    conn = sqlite3.connect(path)
    cur = conn.execute("SELECT * FROM cookies WHERE name = 'api' LIMIT 1")
    row = cur.fetchone()
    cols = [d[0] for d in cur.description] if cur.description else []
    conn.close()
    if not row:
        print("No 'api' cookie found", file=sys.stderr)
        sys.exit(1)
    enc = None
    if 'encrypted_value' in cols:
        enc = row[cols.index('encrypted_value')]
    if not enc and 'value' in cols:
        enc = row[cols.index('value')]
    if not enc:
        print("Cookie value empty", file=sys.stderr)
        sys.exit(1)
    if isinstance(enc, bytes) and (enc.startswith(b'v10') or enc.startswith(b'v11')):
        print(decrypt_linux_chrome(enc))
    else:
        print(enc.decode('utf-8', errors='replace') if isinstance(enc, bytes) else enc)

if __name__ == '__main__':
    main()
