# Nix Infrastructure

Dendritic flake-parts setup for managing my machines and for portable and synced development environment

# Running

## Prerequisites

- `nix`
  - For platforms with non-root access or immutable rootfs good alternatives can be [nix-user-chroot](https://github.com/nix-community/nix-user-chroot) or [nix-portable](https://github.com/DavHau/nix-portable) or similiar.

## Example

### Regular `nix`

```sh
nix run github:tym2k1/nix-infra#myCli
```

### `nix-user-chrot`

```sh
#!/usr/bin/env bash
set -euo pipefail

~/.nix/nix-user-chroot ~/.nix bash -lc \
  'nix --extra-experimental-features "nix-command flakes" run github:tym2k1/nix-infra#myCli'
```
