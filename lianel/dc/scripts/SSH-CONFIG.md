# SSH Configuration for Remote Host Access

## Quick Connect Command

To connect to the remote host (72.60.80.84), use:

```bash
ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84
```

## SSH Key Details

- **Key File**: `~/.ssh/id_ed25519_host`
- **Key Type**: ED25519
- **Remote Host**: `72.60.80.84`
- **Remote User**: `root`

## SSH Options Explained

- `-F /dev/null`: Don't use SSH config file
- `-o UserKnownHostsFile=/dev/null`: Don't save host keys
- `-o StrictHostKeyChecking=no`: Don't prompt for host key verification
- `-o IdentitiesOnly=yes`: Only use the specified identity file
- `-i ~/.ssh/id_ed25519_host`: Use this specific key file

## Common Commands

### Pull latest code and trigger DAG
```bash
ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84 "cd /root/hosting-base && git pull origin master && bash lianel/dc/scripts/trigger-entsoe-dag-remote.sh"
```

### Run any command on remote host
```bash
ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84 "<command>"
```

### Copy file to remote host
```bash
scp -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host <local_file> root@72.60.80.84:<remote_path>
```

### Copy file from remote host
```bash
scp -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84:<remote_file> <local_path>
```

## Alias (Optional)

Add to `~/.bashrc` or `~/.zshrc` for convenience:

```bash
alias ssh-lianel='ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84'
```

Then use: `ssh-lianel "<command>"`
