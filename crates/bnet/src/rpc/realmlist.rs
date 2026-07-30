//! Realm-list payload construction for `GameUtilities.ProcessClientRequest`.
//!
//! The realm list, character counts, and realm-join server addresses are all **JSON documents**
//! (not protobuf) that the client expects compressed a particular way:
//!
//! * each document is a UTF-8 string prefixed with a type tag (`JSONRealmListUpdates:` etc.),
//! * compressed as `[u32 little-endian uncompressed-length][zlib bytes]`, where the uncompressed
//!   length **includes a trailing NUL**,
//! * then handed back as the `blob_value` of a named response attribute.
//!
//! The JSON field names use the client-required camelCase spelling, serialized here by hand with
//! `serde_json` so we do not need a protobuf-JSON layer.

use std::io::Write;

use anyhow::{Context, Result};
use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde_json::json;

/// Region and site (a.k.a. battlegroup) we place every realm under. A single-region deployment
/// only ever needs one sub-region, [`SUBREGION`].
const REGION: u32 = 1;
const SITE: u32 = 1;

/// The one sub-region address we advertise, `"<region>-<site>-0"`, matching
/// `RealmHandle{REGION, SITE, 0}.GetAddressString()`.
pub const SUBREGION: &str = "1-1-0";

/// The version the client reported in its realm-list ticket. Echoed back as each realm's version
/// so the client never sees a build mismatch it would flag.
#[derive(Debug, Clone)]
pub struct ClientVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub build: u32,
}

impl Default for ClientVersion {
    fn default() -> Self {
        // A plausible 1.14 fallback if the client did not send a parseable version.
        Self {
            major: 1,
            minor: 14,
            revision: 2,
            build: 0,
        }
    }
}

/// The bits of a realm needed to render one `RealmEntry`.
#[derive(Debug, Clone)]
pub struct RealmDescriptor {
    pub id: u32,
    pub name: String,
    /// External IP/host the client connects to.
    pub address: String,
    pub port: u16,
    /// Realm timezone/category id (`cfgCategoriesID`).
    pub timezone: u32,
    /// Realm type/config id (`cfgConfigsID`) — PvP/PvE/RP flavour.
    pub config_id: u32,
}

/// The `WoW` realm address: `region << 24 | site << 16 | realm_id` (the low 16 bits carry the
/// realm id).
pub fn wow_realm_address(realm_id: u32) -> u32 {
    (REGION << 24) | (SITE << 16) | (realm_id & 0xFFFF)
}

/// Recover the realm id from a `Param_RealmAddress` value (its low 16 bits).
pub fn realm_id_from_address(addr: u64) -> u32 {
    (addr as u32) & 0xFFFF
}

/// Compress a `prefix:json` document the way the client expects: a little-endian u32 giving the
/// uncompressed length **including a trailing NUL**, followed by the zlib stream of the bytes plus
/// that NUL.
pub fn compress_json(prefixed: &str) -> Result<Vec<u8>> {
    let mut data = prefixed.as_bytes().to_vec();
    data.push(0); // Include the NUL terminator in the compressed data
    let uncompressed_len = data.len() as u32;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&data)
        .context("zlib write for realm-list payload")?;
    let compressed = encoder
        .finish()
        .context("zlib finish for realm-list payload")?;

    let mut out = Vec::with_capacity(4 + compressed.len());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn realm_entry_json(realm: &RealmDescriptor, version: &ClientVersion) -> serde_json::Value {
    json!({
        "wowRealmAddress": wow_realm_address(realm.id),
        "cfgTimezonesID": 1,
        "populationState": 1, // Low — we do not model live population here.
        "cfgCategoriesID": realm.timezone,
        "version": {
            "versionMajor": version.major,
            "versionMinor": version.minor,
            "versionRevision": version.revision,
            "versionBuild": version.build,
        },
        "cfgRealmsID": realm.id,
        "flags": 0,
        "name": realm.name,
        "cfgConfigsID": realm.config_id,
        "cfgLanguagesID": 1,
        "cfgContentSetID": 0,
        "useBleepChance": 0.0,
    })
}

/// Build the compressed `Param_RealmList` blob: a `RealmListUpdates` document listing every realm
/// as a non-deleting update.
pub fn build_realm_list(realms: &[RealmDescriptor], version: &ClientVersion) -> Result<Vec<u8>> {
    let updates: Vec<serde_json::Value> = realms
        .iter()
        .map(|realm| {
            json!({
                "wowRealmAddress": wow_realm_address(realm.id),
                "update": realm_entry_json(realm, version),
                "deleting": false,
            })
        })
        .collect();

    let json = format!(
        "JSONRealmListUpdates:{}",
        serde_json::to_string(&json!({ "updates": updates }))?
    );
    compress_json(&json)
}

/// Build the compressed `Param_CharacterCountList` blob from `(wow_realm_address, count)` pairs.
pub fn build_character_counts(counts: &[(u32, u32)]) -> Result<Vec<u8>> {
    let entries: Vec<serde_json::Value> = counts
        .iter()
        .map(|(addr, count)| json!({ "wowRealmAddress": addr, "count": count }))
        .collect();

    let json = format!(
        "JSONRealmCharacterCountList:{}",
        serde_json::to_string(&json!({ "counts": entries }))?
    );
    compress_json(&json)
}

/// Build the compressed `Param_ServerAddresses` blob pointing the client at one world server.
pub fn build_server_addresses(ip: &str, port: u16) -> Result<Vec<u8>> {
    let family = if ip.contains(':') { 2 } else { 1 }; // 2 = IPv6, 1 = IPv4.
    let doc = json!({
        "families": [{
            "family": family,
            "addresses": [{ "ip": ip, "port": port }],
        }],
    });
    let json = format!(
        "JSONRealmListServerIPAddresses:{}",
        serde_json::to_string(&doc)?
    );
    compress_json(&json)
}

/// Build the `Param_RealmJoinTicket` blob: a plain (uncompressed) `RealmJoinTicket` JSON document.
pub fn build_join_ticket(account: &str, platform: u32, arch: u32, ty: u32) -> Result<Vec<u8>> {
    let doc = json!({
        "gameAccount": account,
        "platform": platform,
        "type": ty,
        "clientArch": arch,
    });
    Ok(serde_json::to_string(&doc)?.into_bytes())
}

/// The subset of the client's realm-list ticket `ClientInfo` we need: the 32-byte client secret
/// (first half of the world session key) and the reported client version.
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub secret: [u8; 32],
    pub version: ClientVersion,
    pub platform: u32,
    pub arch: u32,
    pub ty: u32,
}

/// Parse the `Param_ClientInfo` blob (a `prefix:json` document) into a [`ClientInfo`]. Returns
/// `None` if the secret is missing or not 32 bytes.
pub fn parse_client_info(blob: &[u8]) -> Option<ClientInfo> {
    // Lossy rather than strict: a stray non-UTF-8 byte anywhere in the blob must not cost us the
    // whole ticket, and every field we read is ASCII.
    let text = String::from_utf8_lossy(blob);
    let json = &text[text.find(':')? + 1..];

    // Read one JSON value and ignore whatever follows. `from_str` rejects trailing bytes, and
    // these documents are conventionally NUL-terminated (the compressed data includes the
    // NUL terminator), so a terminator or padding would otherwise fail the whole parse.
    let value: serde_json::Value = serde_json::Deserializer::from_str(json)
        .into_iter()
        .next()?
        .ok()?;
    let info = value.get("info")?;

    let secret = parse_secret(info.get("secret")?)?;

    let v = info.get("version");
    let version = ClientVersion {
        major: json_u32(v, "versionMajor").unwrap_or(1),
        minor: json_u32(v, "versionMinor").unwrap_or(14),
        revision: json_u32(v, "versionRevision").unwrap_or(0),
        build: json_u32(v, "versionBuild").unwrap_or(0),
    };

    Some(ClientInfo {
        secret,
        version,
        platform: json_u32(Some(info), "platformType").unwrap_or(0),
        arch: json_u32(Some(info), "clientArch").unwrap_or(0),
        ty: json_u32(Some(info), "type").unwrap_or(0),
    })
}

/// Read the 32-byte client secret out of `info.secret`.
///
/// The 1.14 client sends it as a **JSON array of 32 numbers**, not a base64 string. Elements may
/// be signed (a byte over 127 arrives negative), so each is masked down rather than range-checked.
/// A base64 string is also accepted for interoperability.
fn parse_secret(value: &serde_json::Value) -> Option<[u8; 32]> {
    let bytes: Vec<u8> = match value {
        serde_json::Value::Array(elements) => elements
            .iter()
            .map(|e| e.as_i64().map(|n| (n & 0xFF) as u8))
            .collect::<Option<Vec<u8>>>()?,
        serde_json::Value::String(encoded) => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?,
        _ => return None,
    };
    bytes.try_into().ok()
}

fn json_u32(value: Option<&serde_json::Value>, key: &str) -> Option<u32> {
    value?.get(key)?.as_u64().map(|n| n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    /// Decompress a `compress_json` blob back to its `prefix:json` string, checking the framing.
    fn decompress(blob: &[u8]) -> String {
        let declared = u32::from_le_bytes(blob[..4].try_into().unwrap()) as usize;
        let mut decoder = ZlibDecoder::new(&blob[4..]);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).unwrap();
        assert_eq!(
            out.len(),
            declared,
            "declared length includes the trailing NUL"
        );
        assert_eq!(out.last(), Some(&0u8), "payload ends with a NUL");
        String::from_utf8(out[..out.len() - 1].to_vec()).unwrap()
    }

    #[test]
    fn wow_realm_address_packs_region_site_and_id() {
        assert_eq!(wow_realm_address(1), 0x0101_0001);
        assert_eq!(wow_realm_address(5), 0x0101_0005);
        assert_eq!(realm_id_from_address(0x0101_0005), 5);
    }

    #[test]
    fn realm_list_round_trips_through_zlib_with_expected_shape() {
        let realms = vec![RealmDescriptor {
            id: 1,
            name: "Oxcore".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8085,
            timezone: 8,
            config_id: 0,
        }];
        let version = ClientVersion {
            major: 1,
            minor: 14,
            revision: 2,
            build: 42597,
        };
        let blob = build_realm_list(&realms, &version).unwrap();
        let text = decompress(&blob);
        assert!(text.starts_with("JSONRealmListUpdates:"));

        let value: serde_json::Value =
            serde_json::from_str(&text["JSONRealmListUpdates:".len()..]).unwrap();
        let update = &value["updates"][0];
        assert_eq!(update["deleting"], json!(false));
        assert_eq!(update["update"]["name"], json!("Oxcore"));
        assert_eq!(update["update"]["wowRealmAddress"], json!(0x0101_0001));
        assert_eq!(update["update"]["version"]["versionBuild"], json!(42597));
    }

    #[test]
    fn character_counts_serialize_with_camelcase_keys() {
        let blob = build_character_counts(&[(wow_realm_address(1), 3)]).unwrap();
        let text = decompress(&blob);
        assert!(text.starts_with("JSONRealmCharacterCountList:"));
        let value: serde_json::Value =
            serde_json::from_str(&text["JSONRealmCharacterCountList:".len()..]).unwrap();
        assert_eq!(value["counts"][0]["count"], json!(3));
        assert_eq!(value["counts"][0]["wowRealmAddress"], json!(0x0101_0001));
    }

    #[test]
    fn server_addresses_pick_ipv4_family() {
        let blob = build_server_addresses("192.0.2.10", 8085).unwrap();
        let text = decompress(&blob);
        assert!(text.starts_with("JSONRealmListServerIPAddresses:"));
        let value: serde_json::Value =
            serde_json::from_str(&text["JSONRealmListServerIPAddresses:".len()..]).unwrap();
        assert_eq!(value["families"][0]["family"], json!(1));
        assert_eq!(
            value["families"][0]["addresses"][0]["ip"],
            json!("192.0.2.10")
        );
        assert_eq!(value["families"][0]["addresses"][0]["port"], json!(8085));
    }

    #[test]
    fn join_ticket_is_plain_json() {
        let bytes = build_join_ticket("PLAYER#1", 0, 0, 0).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["gameAccount"], json!("PLAYER#1"));
    }

    #[test]
    fn client_info_parses_secret_and_version() {
        let secret = [7u8; 32];
        let secret_b64 = base64::engine::general_purpose::STANDARD.encode(secret);
        let blob = format!(
            "JSONRealmListTicketClientInformation:{{\"info\":{{\"secret\":\"{secret_b64}\",\
             \"platformType\":1,\"clientArch\":1,\"type\":2,\
             \"version\":{{\"versionMajor\":1,\"versionMinor\":14,\"versionRevision\":2,\"versionBuild\":42597}}}}}}"
        );
        let info = parse_client_info(blob.as_bytes()).unwrap();
        assert_eq!(info.secret, secret);
        assert_eq!(info.version.build, 42597);
        assert_eq!(info.ty, 2);
    }

    #[test]
    fn client_info_parses_a_secret_sent_as_a_number_array() {
        // How the live 1.14.2 client actually sends it — and with a value past 127 arriving
        // signed, as a C# `List<int>` cast from a byte would.
        let mut numbers: Vec<i32> = (0..32).collect();
        numbers[0] = -1; // 0xFF seen as a signed byte
        let json = format!(
            "JSONRealmListTicketClientInformation:{{\"info\":{{\"secret\":{},\
             \"platformType\":1,\"clientArch\":1,\"type\":2}}}}",
            serde_json::to_string(&numbers).unwrap()
        );

        let info = parse_client_info(json.as_bytes()).unwrap();
        assert_eq!(info.secret[0], 0xFF);
        assert_eq!(info.secret[31], 31);
    }

    #[test]
    fn client_info_rejects_a_number_array_of_the_wrong_length() {
        let json = "x:{\"info\":{\"secret\":[1,2,3]}}";
        assert!(parse_client_info(json.as_bytes()).is_none());
    }

    #[test]
    fn client_info_rejects_a_wrong_length_secret() {
        let secret_b64 = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        let blob = format!("x:{{\"info\":{{\"secret\":\"{secret_b64}\"}}}}");
        assert!(parse_client_info(blob.as_bytes()).is_none());
    }
}
