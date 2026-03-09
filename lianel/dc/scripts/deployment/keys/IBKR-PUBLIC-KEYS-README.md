# IBKR public key PEMs for upload

Use these for **uploading to the IBKR OAuth Self Service portal** (public signing key and public encryption key).

## Option A: You have the **private** key PEMs

From the **private** signature and encryption PEMs (e.g. from IBKR or your server), derive the public PEMs:

```bash
# Put private_signature.pem and private_encryption.pem in this folder, then:
cd "$(dirname "$0")/.."
./derive-ibkr-public-keys.sh keys
```

Output in `keys/`:
- **`public_signing_key.pem`** – upload as “Public signing key” in IBKR.
- **`public_encryption_key.pem`** – upload as “Public encryption key” in IBKR.

## Option B: You have public keys in **SSH format**

If your “public signing key” or “public encryption key” is in SSH format (e.g. `ssh-rsa AAAA...` in one line), convert to PEM:

```bash
# Write the SSH-format key to a file (one line), then:
ssh-keygen -e -m PEM -f public_signing_key_ssh.txt -o public_signing_key.pem
ssh-keygen -e -m PEM -f public_encryption_key_ssh.txt -o public_encryption_key.pem
```

Then upload the resulting `.pem` files to IBKR.

## Note

For IBKR OAuth, the **public** keys must be the ones that match the **private** keys your app uses (private_signature.pem and private_encryption.pem). The SSH host key from the server is **not** the same as these; use the keys from the same key pair as your private PEMs.
