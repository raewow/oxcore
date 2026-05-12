use crate::auth::common::codes::AuthCmd;
use bytes::{Buf, BufMut, BytesMut};

/// Client logon challenge packet (C -> S)
#[derive(Debug)]
pub struct AuthLogonChallengeC {
    pub cmd: u8,
    pub error: u8,
    pub size: u16,
    pub gamename: [u8; 4],
    pub version1: u8,
    pub version2: u8,
    pub version3: u8,
    pub build: u16,
    pub platform: [u8; 4],
    pub os: [u8; 4],
    pub country: [u8; 4],
    pub timezone_bias: u32,
    pub ip: u32,
    pub username: String,
}

impl AuthLogonChallengeC {
    pub fn read_from(buf: &mut BytesMut) -> Result<Self, String> {
        if buf.len() < 4 {
            return Err("Not enough data for header".to_string());
        }

        let cmd = buf.get_u8();
        let error = buf.get_u8();
        let size = buf.get_u16_le();

        if buf.len() < (size as usize) {
            return Err("Not enough data for full packet".to_string());
        }

        let mut gamename = [0u8; 4];
        buf.copy_to_slice(&mut gamename);

        let version1 = buf.get_u8();
        let version2 = buf.get_u8();
        let version3 = buf.get_u8();
        let build = buf.get_u16_le();

        let mut platform = [0u8; 4];
        buf.copy_to_slice(&mut platform);

        let mut os = [0u8; 4];
        buf.copy_to_slice(&mut os);

        let mut country = [0u8; 4];
        buf.copy_to_slice(&mut country);

        let timezone_bias = buf.get_u32_le();
        let ip = buf.get_u32_le();

        let i_len = buf.get_u8();
        if buf.len() < i_len as usize {
            return Err("Not enough data for username".to_string());
        }

        let username_bytes = buf.copy_to_bytes(i_len as usize);
        let username = String::from_utf8(username_bytes.to_vec())
            .map_err(|_| "Invalid UTF-8 in username".to_string())?;

        Ok(AuthLogonChallengeC {
            cmd,
            error,
            size,
            gamename,
            version1,
            version2,
            version3,
            build,
            platform,
            os,
            country,
            timezone_bias,
            ip,
            username,
        })
    }
}

/// Server logon challenge response (S -> C)
pub struct AuthLogonChallengeS {
    pub cmd: u8,
    pub error: u8,
    pub result: u8,
    pub b: [u8; 32],
    pub g_len: u8,
    pub g: u8,
    pub n_len: u8,
    pub n: [u8; 32],
    pub s: Vec<u8>,
    pub unk3: [u8; 16],
    pub security_flags: Option<u8>,
    pub grid_seed: Option<u32>,
    pub server_security_salt: Option<[u8; 16]>,
}

impl AuthLogonChallengeS {
    pub fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.cmd);
        buf.put_u8(self.error);
        buf.put_u8(self.result);
        buf.put_slice(&self.b);
        buf.put_u8(self.g_len);
        buf.put_u8(self.g);
        buf.put_u8(self.n_len);
        buf.put_slice(&self.n);
        buf.put_slice(&self.s);
        buf.put_slice(&self.unk3);

        if let Some(flags) = self.security_flags {
            buf.put_u8(flags);
            if let Some(seed) = self.grid_seed {
                buf.put_u32_le(seed);
            }
            if let Some(salt) = self.server_security_salt {
                buf.put_slice(&salt);
            }
        } else {
            // For builds >= 5428, send 0 if no security flags
            buf.put_u8(0);
        }
    }
}

/// Client logon proof packet (C -> S)
#[derive(Debug)]
pub struct AuthLogonProofC {
    pub cmd: u8,
    pub a: [u8; 32],
    pub m1: [u8; 20],
    pub crc_hash: [u8; 20],
    pub number_of_keys: u8,
    pub security_flags: Option<u8>,
    pub pin_data: Option<crate::auth::auth::pin::PinData>,
}

impl AuthLogonProofC {
    pub fn read_from(buf: &mut BytesMut, build: u16) -> Result<Self, String> {
        if buf.len() < 73 {
            return Err("Not enough data for logon proof base".to_string());
        }

        let cmd = buf.get_u8();

        let mut a = [0u8; 32];
        buf.copy_to_slice(&mut a);

        let mut m1 = [0u8; 20];
        buf.copy_to_slice(&mut m1);

        let mut crc_hash = [0u8; 20];
        buf.copy_to_slice(&mut crc_hash);

        let number_of_keys = buf.get_u8();

        let security_flags = if build >= 5428 {
            if buf.len() < 1 {
                return Err("Not enough data for security flags".to_string());
            }
            Some(buf.get_u8())
        } else {
            None
        };

        // Read PIN data if security flags are set
        let pin_data = if security_flags.is_some() && security_flags.unwrap() != 0 {
            if buf.len() < 36 {
                return Err("Not enough data for PIN data".to_string());
            }
            let mut salt = [0u8; 16];
            buf.copy_to_slice(&mut salt);
            let mut hash = [0u8; 20];
            buf.copy_to_slice(&mut hash);
            Some(crate::auth::auth::pin::PinData { salt, hash })
        } else {
            None
        };

        Ok(AuthLogonProofC {
            cmd,
            a,
            m1,
            crc_hash,
            number_of_keys,
            security_flags,
            pin_data,
        })
    }
}

/// Server logon proof response (S -> C)
pub struct AuthLogonProofS {
    pub cmd: u8,
    pub error: u8,
    pub m2: [u8; 20],
    pub account_flags: Option<u32>, // For build >= 8089
    pub survey_id: Option<u32>,
    pub login_flags: Option<u16>,
}

impl AuthLogonProofS {
    pub fn write_to(&self, buf: &mut BytesMut, build: u16) {
        buf.put_u8(self.cmd);
        buf.put_u8(self.error);
        buf.put_slice(&self.m2);

        // For vanilla (build < 6299): always include surveyId (4 bytes, set to 0)
        if build < 6299 {
            let survey_id = self.survey_id.unwrap_or(0);
            buf.put_u32_le(survey_id);
        }
        // For TBC (build >= 6299 && < 8089): include surveyId and loginFlags
        else if build < 8089 {
            let survey_id = self.survey_id.unwrap_or(0);
            buf.put_u32_le(survey_id);
            let login_flags = self.login_flags.unwrap_or(0);
            buf.put_u16_le(login_flags);
        }
        // For WotLK+ (build >= 8089): include accountFlags, surveyId, and loginFlags
        else {
            if let Some(flags) = self.account_flags {
                buf.put_u32_le(flags);
            }
            let survey_id = self.survey_id.unwrap_or(0);
            buf.put_u32_le(survey_id);
            let login_flags = self.login_flags.unwrap_or(0);
            buf.put_u16_le(login_flags);
        }
    }
}

/// XFER_INIT packet structure (S -> C)
/// Matches the C++ XFER_INIT structure
#[derive(Debug)]
pub struct XferInit {
    pub cmd: u8,            // XFER_INITIATE
    pub file_name_len: u8,  // strlen(fileName) = 5
    pub file_name: [u8; 5], // "Patch" (but first byte is 0, then 0x05, then "Patch")
    pub file_size: u64,     // file size (bytes)
    pub md5: [u8; 16],      // MD5 hash
}

impl XferInit {
    pub fn new(file_size: u64, md5: [u8; 16]) -> Self {
        // The C++ code does: memcpy(&xferh, "0\x05Patch", 7);
        // This means: [0, 0x05, 'P', 'a', 't', 'c', 'h']
        // But the struct only has 5 bytes for fileName, so it's: [0, 0x05, 'P', 'a', 't']
        let mut file_name = [0u8; 5];
        file_name[0] = 0;
        file_name[1] = 0x05;
        file_name[2] = b'P';
        file_name[3] = b'a';
        file_name[4] = b't';

        Self {
            cmd: AuthCmd::XferInitiate as u8,
            file_name_len: 5,
            file_name,
            file_size,
            md5,
        }
    }

    pub fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.cmd);
        buf.put_u8(self.file_name_len);
        buf.put_slice(&self.file_name);
        buf.put_u64_le(self.file_size);
        buf.put_slice(&self.md5);
    }
}

/// Client reconnect proof packet (C -> S)
/// C++: sAuthReconnectProof_C
#[derive(Debug)]
pub struct AuthReconnectProofC {
    pub cmd: u8,
    pub r1: [u8; 16],
    pub r2: [u8; 20],
    pub r3: [u8; 20], // version proof (integrity check)
    pub number_of_keys: u8,
}

impl AuthReconnectProofC {
    pub fn read_from(buf: &mut BytesMut) -> Result<Self, String> {
        if buf.len() < 57 {
            return Err("Not enough data for reconnect proof".to_string());
        }

        let cmd = buf.get_u8();

        let mut r1 = [0u8; 16];
        buf.copy_to_slice(&mut r1);

        let mut r2 = [0u8; 20];
        buf.copy_to_slice(&mut r2);

        let mut r3 = [0u8; 20];
        buf.copy_to_slice(&mut r3);

        let number_of_keys = buf.get_u8();

        Ok(AuthReconnectProofC {
            cmd,
            r1,
            r2,
            r3,
            number_of_keys,
        })
    }
}

/// Server reconnect challenge response (S -> C)
pub struct AuthReconnectChallengeS {
    pub cmd: u8,
    pub error: u8,
    pub reconnect_proof: [u8; 16],   // 16 bytes random
    pub version_challenge: [u8; 16], // VERSION_CHALLENGE
}

impl AuthReconnectChallengeS {
    pub fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.cmd);
        buf.put_u8(self.error);
        buf.put_slice(&self.reconnect_proof);
        buf.put_slice(&self.version_challenge);
    }
}

/// Server reconnect proof response (S -> C)
pub struct AuthReconnectProofS {
    pub cmd: u8,
    pub error: u8,
}

impl AuthReconnectProofS {
    pub fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.cmd);
        buf.put_u8(self.error);
    }
}
