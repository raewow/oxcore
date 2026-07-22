# oxcore-patcher — point a 1.14.x client at your server

A 1.14 WoW client pins its Battle.net connection to a certificate bundle baked into the executable,
so it will not trust a self-hosted server's TLS certificate. This tool rewrites the relevant bytes
**on disk** (never in place by default) so the client trusts *your* certificate and resolves *your*
host. No memory injection, no process spawning — it reads an executable and writes a patched copy.

For the full operator + player flow, see the [`oxcore-bnet` README](../bnet/README.md). This file
documents the tool itself.

## What it patches

| Patch | What it does | Source |
|-------|--------------|--------|
| **portal suffix** | rewrites `.actual.battle.net` → your suffix (default `.localhost`), NUL-padded | `--portal` |
| **signature modulus** | replaces the 256-byte RSA modulus the client verifies the cert bundle with | `--modulus` (required) |
| **cert bundle** | replaces the embedded JSON bundle so it lists *your* certificate | `--cert-bundle` (required) |

All three replacements are **length-preserving** (NUL-padded), so no offsets shift. Every pattern
must match **exactly once** or the run aborts — a silent zero- or multi-match is how a patcher
corrupts a client. The `--modulus` and `--cert-bundle` blobs come as a matched pair from
`bnet gen-certs`; using the modulus without the bundle (or vice versa) leaves the client trusting
Blizzard's certificates, so both are required.

## Usage

```sh
# Generate the matched cert + patch artifacts first (see the bnet README):
cargo run -p oxcore-bnet --bin bnet -- gen-certs --out ./certs --host oxcore.localhost

# Preview every change and its offset, writing nothing:
oxcore-patcher -i WowClassic.exe \
  --modulus ./certs/signature_modulus.bin \
  --cert-bundle ./certs/cert_bundle.bin \
  --dry-run

# Apply — writes WowClassic.exe.patched (override with -o):
oxcore-patcher -i WowClassic.exe \
  --modulus ./certs/signature_modulus.bin \
  --cert-bundle ./certs/cert_bundle.bin
```

| Flag | Default | Meaning |
|------|---------|---------|
| `-i, --input` | — | client executable to read |
| `-o, --output` | `<input>.patched` | where to write the patched copy |
| `--portal` | `.localhost` | suffix that replaces `.actual.battle.net`; must be **≤** its length and start with a dot |
| `--modulus` | *(required)* | `signature_modulus.bin` from `bnet gen-certs` |
| `--cert-bundle` | *(required)* | `cert_bundle.bin` from `bnet gen-certs` |
| `--dry-run` | off | list matches and offsets, write nothing |

## After patching

The connect host the client resolves is `<portal cvar>` + the patched suffix. Set the cvar in
`WTF/Config.wtf` and make that host resolve to your server:

```
# WTF/Config.wtf
SET portal "oxcore"          # → connects to oxcore.localhost

# /etc/hosts  (or C:\Windows\System32\drivers\etc\hosts)
127.0.0.1   oxcore.localhost
```

That host must match the certificate's `--host` and the server's `external_hostname`, or the client
drops the connection with no error. Then launch the patched executable.

## Scope

Only the portal suffix and signature modulus are strictly required for 1.14.x. The connect-to
modulus, cert-bundle JSON marker, and version/CDN URL patterns are kept as constants in
`patterns.rs` for later builds (2.5/3.4/4.4) and the world-side `SMSG_CONNECT_TO` work, but are not
applied here.
