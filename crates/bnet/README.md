# oxcore-bnet — Battle.net login server for 1.14.x clients

`bnet` lets a **modern Classic client (1.14.x)** log in to an oxcore server. It runs *alongside*
`auth` (which keeps serving the 1.12 vanilla client on the legacy realmd protocol); the two share
only the account database.

A 1.14 client does not speak realmd. It authenticates against Battle.net (BGS): an HTTPS REST
login, then a TLS protobuf-RPC channel that carries logon, the realm list, and realm join. Because
that channel is TLS-pinned to a certificate bundle baked into the client, the client must be
**patched** to trust *your* certificate. That is what [`oxcore-patcher`](../patcher) does.

> **Status.** The login pipeline is implemented end to end — login → logon → realm list → realm
> join — and every layer is unit-tested for internal correctness. It has **not** been verified
> against a live retail client, and the world-side handshake a 1.14 client performs *after* realm
> join is not built yet (see [Part C](#what-works-and-what-doesnt)). Expect the client to reach the
> realm screen and then fail at world-connect.

---

## How a patched client reaches you

```
                         :8081 (HTTPS REST)                 :1119 (TLS protobuf-RPC)
  ┌────────┐  portal/    ┌───────────────┐  login ticket   ┌────────────────────────┐
  │ client │ ──────────▶ │  REST login   │ ──────────────▶ │  BGS RPC channel        │
  │        │  login/srp/ │  (SRP6v2)     │                 │  Connect → Logon        │
  │        │ ◀────────── │               │                 │  → VerifyWebCredentials │
  └────────┘   M2+ticket └───────────────┘                 │  → RealmList → Join     │
       │                                                    └────────────────────────┘
       │  world address + session key (from realm join)                 │
       └───────────────────────  NOT YET HANDLED (Part C)  ──────────────┘
```

The connect host the client resolves is **`<portal>` + `<suffix>`**, where `<portal>` is the
`portal` cvar in `WTF/Config.wtf` and `<suffix>` is the string the patcher rewrites (default
`.localhost`, from Blizzard's `.actual.battle.net`). That host must be identical in three places:

1. the certificate you generate (`gen-certs --host`),
2. the server's `external_hostname` config,
3. `<portal>` + the patched suffix.

If they disagree, the client silently drops the TLS connection.

---

## Operator setup

### 1. Apply the database migration

The bnet columns live on the shared `account` table:

```
sql/migrations/20260720120000_auth_add_bnet_srp_columns.sql
```

Run your normal migration step against the auth database. New accounts created through the console
get **both** verifiers (vanilla SHA-1 *and* bnet SRP6v2) from one password, so a player logs in
from either client with the same credentials.

### 2. Generate the certificate and patch artifacts

Pick the host the client will resolve — here `oxcore.localhost`:

```sh
cargo run -p oxcore-bnet --bin bnet -- gen-certs --out ./certs --host oxcore.localhost
```

This writes six files to `./certs`:

| File | Used by | Purpose |
|------|---------|---------|
| `bnet.cert.pem` | bnet server | TLS certificate served on both ports |
| `bnet.key.pem` | bnet server | TLS private key |
| `signature_modulus.bin` | patcher `--modulus` | replaces the client's bundle-verify modulus |
| `cert_bundle.bin` | patcher `--cert-bundle` | the signed bundle that trusts `bnet.cert.pem` |
| `connect_to_modulus.bin` | patcher `--connect-to-modulus` | replaces the client's world-signature modulus |
| `world.signing.key.pem` | world server | RSA key that signs the modern world handshake |

Regenerating replaces all six as a matched set — never mix files from different runs. The last two
are for the modern **world** handshake (Part C); a login-only test needs only the first four.

### 3. Configure and run the server

In your `config.toml`, under `[bnet]`:

```toml
[bnet]
login_database_url = "mysql://user:pass@127.0.0.1/oxcore_auth"
external_hostname  = "oxcore.localhost"   # MUST equal the cert host and <portal>+<suffix>
bnet_port          = 1119
login_port         = 8081
cert_file          = "./certs/bnet.cert.pem"
key_file           = "./certs/bnet.key.pem"
# login_ticket_duration = 3600            # seconds a login ticket stays valid
```

Then:

```sh
cargo run -p oxcore-bnet --bin bnet -- --config config.toml
```

It logs the portal address patched clients should use. Run it next to `auth`, not inside it — the
two are separate binaries with separate lifecycles, and an operator may want only one.

### 4. Sanity-check the REST endpoint

```sh
curl --cacert ./certs/bnet.cert.pem https://oxcore.localhost:8081/bnetserver/portal/
# → oxcore.localhost:1119
```

(Resolve `oxcore.localhost` to `127.0.0.1` first — see the player steps below.)

---

## Player steps (patch and connect)

1. **Patch the client** (see [`oxcore-patcher`](../patcher) for detail):

   ```sh
   oxcore-patcher -i WowClassic.exe \
     --modulus ./certs/signature_modulus.bin \
     --cert-bundle ./certs/cert_bundle.bin \
     --connect-to-modulus ./certs/connect_to_modulus.bin   # for the world handshake; omit for login-only
   # writes WowClassic.exe.patched
   ```

   The default `--portal .localhost` suffix produces a connect host of `<portal>.localhost`. To
   target a different suffix, pass `--portal .yourdomain` (it must be no longer than
   `.actual.battle.net` — replacements are length-preserving).

2. **Point the connect host at the server.** Add a hosts-file entry so the OS resolver sends the
   patched host to your server:

   ```
   127.0.0.1   oxcore.localhost
   ```

   (`/etc/hosts` on Linux/macOS, `C:\Windows\System32\drivers\etc\hosts` on Windows.)

3. **Set the portal cvar** in `WTF/Config.wtf` so `<portal>` + `.localhost` = your host:

   ```
   SET portal "oxcore"
   ```

4. **Launch the patched executable** and log in with your account credentials.

You should reach the realm list, and clicking a realm should produce a join response in the bnet
log. Failure at the world-connect step after that is expected today.

---

## What works, and what doesn't

**Implemented (M1–M5):**

- REST login: portal, login form, **SRP6v2** (SHA-256/512) challenge → proof → login ticket.
- BGS RPC framing + `ConnectionService` (Connect / KeepAlive / RequestDisconnect).
- `AuthenticationService.Logon` → `ChallengeListener.OnExternalChallenge` (web-auth URL), and
  `VerifyWebCredentials` → `AuthenticationListener.OnLogonComplete`.
- `GameUtilities.ProcessClientRequest`: realm-list ticket, realm list + character counts
  (compressed JSON), and realm join — which mints the **world session key**
  (`client_secret ++ server_secret`, 64 bytes) and persists it to `account.sessionkey`.

**Not yet (Part C and beyond):** the modern **world-server handshake**. After realm join the
client opens a world connection whose auth differs from vanilla's (different `SMSG_AUTH_CHALLENGE`
shape, different header crypto, a second "instance connect" socket). Nothing in `crates/world`
handles that yet, so the client stops there. After that comes the `to_vanilla`/`to_classic`
message split.

**Unverified against a live client.** Every wire detail is transcribed faithfully from TrinityCore
and CypherCore and is self-consistent under test, but no retail client has exercised it. The likely
first failure points are the certificate signature scheme, the SRP6v2 details, the REST
request-correlation assumption, the RPC framing, the Logon/VerifyWebCredentials callback ordering,
and the realm-list JSON/attribute contract. The client gives no error on a mismatch — it just
disconnects — so budget for packet-capture debugging against a known-good server.
