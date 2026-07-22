# Part C — the modern (1.14.x) world-server handshake (scope)

Status: **scoping only, nothing built.** This document defines the work to make a patched 1.14
client that has completed bnet realm-join actually connect to `crates/world`. It is the seam the
`bnet` crate stops at: realm-join hands the client a world address and a 64-byte session key
(`client_secret ++ server_secret`) persisted to `account.sessionkey`; nothing in `crates/world`
speaks the protocol the client then uses.

Everything here is transcribed from HermesProxy's modern server `WorldSocket.cs` /
`PacketCrypt.cs` and TrinityCore. **None of it is verified against a live client**, same boundary as
the rest of the bnet work.

---

## Why this is a separate protocol, not a tweak

The 1.12 path (`crates/world/src/core/network/`) is a different protocol family end to end:

| | 1.12 (implemented) | 1.14 (this doc) |
|---|---|---|
| Transport | raw TCP | raw TCP (no TLS here — TLS was only bnet) |
| Connect preamble | none | plaintext `"WORLD OF WARCRAFT CONNECTION - … - V2"` exchange |
| Server header | `[u16 size BE][u16 opcode LE]` (4B) | `[u32 size LE][12B GCM tag]` (16B) |
| Client header | `[u16 size BE][u32 opcode LE]` (6B) | same 16B framing both ways |
| Header crypt | MaNGOS RC4-drop (`crypt.rs`) | **AES-128-GCM** over `opcode‖body` |
| Auth challenge | `u32` server seed | `Challenge[16] ++ DosChallenge[32] ++ DosZeroBits u8` |
| Auth proof | 20-byte SHA-1 digest | 32-byte HMAC-SHA-256 digest |
| Session key | vanilla SRP6 (SHA-1) | bnet realm-join key (64B) |
| Encrypt handoff | implicit after proof | explicit `SMSG_ENTER_ENCRYPTED_MODE` (RSA-signed) + ack |
| Instance handoff | none | `SMSG_CONNECT_TO` to a second socket |

So Part C is a parallel network stack under `crates/world`, selected per-connection, not an edit to
the existing one. The existing vanilla stack must keep working untouched.

---

## The handshake, step by step

1. **Client connects** to the world address from realm-join. **No TLS.**
2. **Connection init (plaintext).** Server sends `"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2"`;
   client replies `"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2"`. (Exact terminator/length
   to confirm against a capture.)
3. **`SMSG_AUTH_CHALLENGE`** (server → client, unencrypted): a 16-byte `serverChallenge`, a 32-byte
   `DosChallenge`, and a `DosZeroBits` byte (HermesProxy uses `1`).
4. **`CMSG_AUTH_SESSION`** (client → server, unencrypted): `Build`, `RegionID`, `BattlegroupID`,
   `RealmID`, `RealmJoinTicket` (the game-account name string from the join ticket — the correlation
   key back to the account), `LocalChallenge[16]`, `Digest[32]`, `DosResponse (u64)`, and
   compressed `AddonInfo`.
5. **Verify the digest** (below). The input session key is the 64-byte bnet realm-join key we
   persisted; look the account up by `RealmJoinTicket`.
6. **Derive the AES-128 key** (below) and send **`SMSG_ENTER_ENCRYPTED_MODE`** — which carries the
   enable flag + key **RSA-signed** with a key whose modulus is baked into the client (the
   `CONNECT_TO_MODULUS` in `crates/patcher/src/patterns.rs`, `91 D5 9B B7 …`). Client replies
   **`CMSG_ENTER_ENCRYPTED_MODE_ACK`**. From here every packet is AES-GCM.
7. **`SMSG_AUTH_RESPONSE`** (success) with realm/character-list enablement, then normal opcode flow
   (char enum, login, etc.).
8. **Entering a map/instance** later triggers **`SMSG_CONNECT_TO`** — a redirect (address + a
   `ConnectToKey`) to a second "instance" socket that repeats a lighter version of this handshake
   (`CMSG_AUTH_CONTINUED_SESSION` using `ContinuedSessionSeed`).

### Digest verification (step 5)

```
seed        = per-(build,OS) auth seed, else the build's fallback static seed
digestKey   = SHA256( SessionKey ‖ seed )
expected    = HMAC-SHA256( key = digestKey,
                           msg = LocalChallenge ‖ serverChallenge ‖ AuthCheckSeed )
accept iff expected == CMSG_AUTH_SESSION.Digest
```

### AES-128 key derivation (step 6)

```
keyData     = SHA256( SessionKey )
prkSeed     = HMAC-SHA256( key = keyData,
                           msg = serverChallenge ‖ LocalChallenge ‖ SessionKeySeed )
sessionKey40 = SessionKeyGenerator(prkSeed, 32).generate(40 bytes)   # SHA256-based expansion (TC "SessionKeyGenerator")
encKeyHmac  = HMAC-SHA256( key = sessionKey40,
                           msg = LocalChallenge ‖ serverChallenge ‖ EncryptionKeySeed )
aesKey      = encKeyHmac[0..16]
```

### Fixed 16-byte seed constants (from HermesProxy/TrinityCore `WorldSocket`)

```
AuthCheckSeed       = C5 C6 98 95 76 3F 1D CD B6 A1 37 28 B3 12 FF 8A
SessionKeySeed      = 58 CB CF 40 FE 2E CE A6 5A 90 B8 01 68 6C 28 0B
ContinuedSessionSeed= 16 AD 0C D4 46 F9 4F B2 EF 7D EA 2A 17 66 4D 2F
EncryptionKeySeed   = E9 75 3C 50 90 93 61 DA 3B 07 EE FA FF 9D 41 B8
```

Per-`(build, OS)` auth seeds + fallback static seeds are a **data table** (HermesProxy ships
`BuildAuthSeeds.csv`); we need the 1.14.x rows for the target build(s) and OS strings
(`Wn64`, `Mc64`, `MacA`).

### Packet crypto (AES-128-GCM)

- Encrypts `opcode(u16) ‖ body` in place; the 4-byte size is plaintext, not authenticated.
- **Nonce (12 bytes)** = `counter(u64 LE) ‖ tagId(u32 LE)`, where `tagId` = `0x52565253` ("SRVR")
  server→client, `0x544E4C43` ("CLNT") client→server. Counters start at 0 and increment **per
  packet**, per direction.
- **Tag** = 12 bytes (GCM tag truncated to fit the 16-byte header alongside the u32 size).
- Header on the wire: `[u32 size LE][12-byte tag]`, then the ciphertext.
- Large packets (`> 0x400` bytes) may be zlib-wrapped as `SMSG_COMPRESSED_PACKET` (adler32 +
  deflate). **Optional** — skip until basic flow works.

---

## What we already have vs. need

**Have (from bnet):** the 64-byte session key persisted at realm-join keyed to the account, and the
`RealmJoinTicket` game-account name that `CMSG_AUTH_SESSION` echoes — so the correlation key and the
crypto input both exist. `crates/patcher` already knows the `CONNECT_TO_MODULUS` pattern.

**Need to build:**
1. A modern world socket (parallel to `socket.rs`): 16-byte GCM framing, the plaintext init
   exchange, AES-GCM read/write with per-direction counters.
2. `WorldCrypt` (AES-128-GCM) — `aes-gcm` crate; keyed by the derived `aesKey`.
3. The key-derivation + digest-verify module (SHA-256, HMAC-SHA-256, the `SessionKeyGenerator`
   expansion, the seed constants, the build/OS seed table).
4. Modern opcodes + (de)serialization for `SMSG_AUTH_CHALLENGE`, `CMSG_AUTH_SESSION`,
   `SMSG_ENTER_ENCRYPTED_MODE`, `CMSG_ENTER_ENCRYPTED_MODE_ACK`, `SMSG_AUTH_RESPONSE`,
   `SMSG_CONNECT_TO`, `CMSG_AUTH_CONTINUED_SESSION`. Modern opcode numbers differ per build and are
   their own reconstruction problem.
5. `SMSG_ENTER_ENCRYPTED_MODE` RSA signing → a **new patcher patch** for `CONNECT_TO_MODULUS` and a
   matching keypair from `bnet gen-certs` (reuse the cert-bundle signing machinery).
6. Account lookup by `RealmJoinTicket` + the 64-byte session key (a shared repo method).

**Explicitly out of scope for Part C:** the `SMSG_CONNECT_TO` instance handoff can be stubbed at
first (single-socket, no instance redirect) to reach character-select; and the
`Message::to_world_packet` → `to_vanilla`/`to_classic` split is the *next* body of work, only worth
doing once a 1.14 client actually reaches gameplay opcodes.

---

## Proposed milestones

- **C1 — modern socket + GCM plumbing. ✅ Done.** `crates/world/src/core/network/modern/`:
  `crypt.rs` (`WorldCrypt`, AES-128-GCM, 12-byte tag, `counter ‖ "SRVR"/"CLNT"` nonce, per-direction
  counters) and `framing.rs` (the `[u32 size][12-byte tag][ciphertext]` codec + the V2 connection-init
  strings). 11 unit tests: both-direction round trips, nonce layout, counter advancement, tamper/
  out-of-order rejection, streaming/partial/concatenated decode, header layout. Added `aes-gcm`
  dep. **Not yet wired into the accept loop** — transport primitives only. The real socket type +
  init exchange land with C3 when there is a handshake to drive them.
- **C2 — key derivation + digest verify. ✅ Done.** `modern/auth_crypto.rs`: the four seed
  constants, `expected_digest`/`verify_digest` (constant-time), `derive_keys` (→ 40-byte
  continued-session key + 16-byte AES key), and the `SessionKeyGenerator` 32-byte-block expansion,
  all transcribed verbatim from HermesProxy `WorldSocket.cs`/`SessionKeyGeneration.cs` — including
  the challenge orderings that differ between the three HMACs. 6 tests: digest self-consistency,
  wrong-key rejection, challenge-order sensitivity, derivation determinism, and the generator's
  behaviour across the 32-byte block boundary (+ a pinned block-1 regression snapshot). The
  account-by-ticket lookup reuses the existing `AccountRepository::find_session_key(username)`
  (username = the join ticket's `gameAccount`), so no new DB code. **Still unverified against a live
  client** — the digest match is the thing only a real client can confirm.
- **C3 — auth handshake packets + encrypted-mode handoff. ✅ Done (crypto/packets), C3b/C4 remaining.**
  `modern/opcodes.rs` (1.14.1/40688 handshake opcodes: `SMSG_AUTH_CHALLENGE=0x3048`,
  `CMSG_AUTH_SESSION=0x3765`, `SMSG_ENTER_ENCRYPTED_MODE=0x3049`, `…ACK=0x3767`,
  `SMSG_AUTH_RESPONSE=0x256D`, …). `modern/packets.rs`: `AuthChallenge` encode (DosChallenge[32] ‖
  Challenge[16] ‖ DosZeroBits), `AuthSession` parse (incl. the 24-byte truncated digest, the
  bit-flushed `UseIPv6`, and JSON `gameAccount` extraction from the realm-join ticket), and
  `SMSG_ENTER_ENCRYPTED_MODE` = `HMAC-SHA256(aes_key; [enabled]‖EnableEncryptionSeed)` signed then a
  flushed enabled-bit — with the RSA behind an `EnterEncryptedModeSigner` trait. `framing.rs` gained
  `encode/decode_plaintext` (the pre-encryption framing: 16-byte header, zero tag, plaintext).
  `modern/handshake.rs`: an I/O-free `HandshakeServer` composing challenge → verify_session (digest
  vs persisted session key) → derive keys → enter-encrypted-mode, with an end-to-end test that
  simulates a correct client and asserts both sides derive the same AES key. Fixed `verify_digest`
  to compare the client's 24-byte digest against the HMAC prefix. 28 modern tests pass.
  **Remaining before a client connects (C3b/C4):** `SMSG_AUTH_RESPONSE` (the bit-packed success
  body), the real RSA `EnterEncryptedModeSigner` + the patcher `CONNECT_TO_MODULUS` patch + a
  gen-certs keypair, the plaintext connection-init exchange, wiring into the accept loop, and the
  per-build/OS auth-seed table.
- **C3b — RSA signer + patcher connect-to modulus. ✅ Done.** `modern/rsa_signer.rs`: `RsaSigner`
  (impl `EnterEncryptedModeSigner`) signs the pre-hash with RSA-PKCS1-v1.5/SHA-256 then **reverses
  the bytes** (little-endian, per HermesProxy `.Reverse()`); loads from a PKCS#1 PEM. Patcher gained
  `patch::connect_to_modulus` (mirrors `signature_modulus`, uses the `CONNECT_TO_MODULUS` prefix)
  and the optional `--connect-to-modulus` flag. `bnet gen-certs` now also mints a **separate** world
  RSA key: `certs::generate_world_signing_key()` writes `world.signing.key.pem` (for the world
  server) and `connect_to_modulus.bin` (the little-endian modulus for the patcher) — the two are
  generated together so the client modulus and the server key cannot drift. Tests: RSA
  sign→reverse→un-reverse→verify (and that the as-sent bytes do *not* verify directly, proving the
  reversal), PEM round-trip, patcher fixture patches at a distinct offset from the signature
  modulus, wrong-size rejection, and the little-endian modulus export. READMEs updated (six
  gen-certs files, the new flag). **Still unverified against a live client**: the signature reversal
  and modulus byte order are faithful to the reference but confirmed only by a real client.
- **C4 — auth-response body + bit writer + auth seeds. ✅ Done (serialization/data), live wiring in C5.**
  `modern/bitbuf.rs`: the bit-packed `BitWriter` (MSB-first bits, byte writes auto-flush pending
  bits) matching CypherCore's `ByteBuffer`. `modern/packets.rs`: `auth_response_success` — the
  minimal `SMSG_AUTH_RESPONSE` success body (result Ok, no queue/realms/templates, optional
  race/class availability), field order + bit-packing per HermesProxy `AuthResponse.Write`.
  `modern/auth_seeds.rs`: the per-(build, OS) digest seeds from `BuildAuthSeeds.csv` (1.14.0/1/2,
  Windows/Mac) with a compile-time hex decoder and `lookup(build, os)`. `handshake::auth_response_frame`
  frames it **encrypted** via `WorldCrypt`. Full-flow test: challenge → session verify → derive keys
  → encrypt SMSG_AUTH_RESPONSE with the derived key → a client-role crypt decodes it. 12 new tests.
- **C5 — auth driver (full handshake over a stream). ✅ Done.** `modern/driver.rs`: `run_auth`
  drives the entire pre-gameplay handshake over any `AsyncRead + AsyncWrite` — the plaintext
  connection-init exchange (server greets first, `\n`-terminated), `SMSG_AUTH_CHALLENGE`, parse +
  verify `CMSG_AUTH_SESSION` (session-key lookup by ticket + `auth_seeds` lookup by build/OS),
  `SMSG_ENTER_ENCRYPTED_MODE`, the **plaintext** ack, then flips `WorldCrypt` on and sends the
  encrypted `SMSG_AUTH_RESPONSE` — returning an `AuthedConnection` (live cipher, account,
  40-byte continued-session key). Account lookup is behind a `SessionKeyProvider` trait
  (`Send` future); the production `AccountSessionKeys` impl reads `account.sessionkey` via the
  shared repo. Timing confirmed from HermesProxy: the ack is plaintext and encryption enables only
  after it. 2 end-to-end tests over a `tokio::io::duplex` with a fully simulated client: a correct
  client completes and gets a decryptable `SMSG_AUTH_RESPONSE`; a wrong session key aborts before
  encrypted mode. 44 modern tests pass; world builds clean.
- **C5b — accept loop + encrypted post-auth loop. ✅ Done.** `modern/driver.rs` gained
  `run_connection` (the post-auth **encrypted** read/dispatch loop) and `serve_connection`
  (`run_auth` then `run_connection`); `CMSG_PING` is answered with `SMSG_PONG`, unknown opcodes are
  logged and skipped. `modern/server.rs`: `serve_modern` — a `TcpListener` accept loop that runs the
  full lifecycle per connection (mirrors bnet's `serve_bnet`), plus `ModernServerConfig` (build/OS,
  virtual-realm address, expansion levels). Account lookup uses the production `AccountSessionKeys`.
  Test: an encrypted `CMSG_PING` over a duplex gets an encrypted `SMSG_PONG` echoing the serial.
  46 modern tests pass; world builds clean.
- **C5c — bootstrap wiring. ✅ Done.** The world `Config` gained opt-in fields (`modern_world_enabled`
  default false, `modern_world_port` 8086, `modern_world_build` 41794, `modern_world_os` `Wn64`,
  `modern_world_signing_key` `./certs/world.signing.key.pem`). `run::serve`, when enabled, loads the
  `RsaSigner` from that PEM, builds a `ModernServerConfig` (build/OS + `virtual_realm_address =
  0x0101_0000 | realm_id`), and spawns `serve_modern` alongside the vanilla listener on the shared
  shutdown broadcast — failing soft (warn + skip) if the key is missing. The whole workspace builds.
  **A live 1.14 client can now actually reach the world server for the first time** — the point every
  prior "unverified" caveat becomes testable. Note the build/opcode-table mismatch to watch: opcodes
  were transcribed from build 40688 but the default seed is build 41794 (`build` must match the real
  client and have a seed entry).
- **C6 — modern gameplay opcodes.** The first real gameplay exchange: `CMSG_ENUM_CHARACTERS` →
  `SMSG_ENUM_CHARACTERS_RESULT` (an empty list reaches the character screen; a populated one needs
  the modern character serialization) and `SMSG_CONNECT_TO` for instance redirects. This is where
  the **`to_vanilla`/`to_classic` message split** becomes unavoidable — every gameplay packet must
  serialize in the client's format.

---

## Risks / unknowns (all need a live-client capture to close)

- **Modern opcode numbers** for the target 1.14 build are unpublished; reconstructing them is a
  parallel effort to the crypto and is the likeliest place to stall (a wrong opcode = silent
  disconnect, no error).
- **Build/OS auth-seed table** must match the exact client build; a mismatch fails the digest and
  the client drops.
- **GCM tag length** (12 vs 16) and whether the size field is AAD — inferred from a 16-byte header;
  confirm against a capture.
- **`SMSG_ENTER_ENCRYPTED_MODE` signature format** (what exactly is signed, padding scheme) and the
  matching `CONNECT_TO_MODULUS` patch — another RSA scheme to get byte-exact.
- **Connection-init string terminator** and whether the server or client sends first.
- This all rides on the bnet realm-join session key being **exactly** what the client used; any
  discrepancy there surfaces here as a digest failure with no diagnostic.
