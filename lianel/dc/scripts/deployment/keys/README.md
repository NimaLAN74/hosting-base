# Remote server SSH host key

- **`remote_host_public_key.txt`** – SSH host public key(s) for the deploy target (e.g. 72.60.80.84), obtained via `ssh-keyscan`.
- Use to pre-populate `~/.ssh/known_hosts` or verify the host key.
- To append to known_hosts: `cat remote_host_public_key.txt >> ~/.ssh/known_hosts`
