//! World server opcodes, carrying one wire number per protocol.
//!
//! The same logical message has a different number on the vanilla 1.12 wire and the modern 1.14
//! wire, so an `Opcode` holds both. A field left at `0` means "this opcode does not exist in that
//! protocol" — which is common in both directions (Battle-Pay has no 1.12 number; plenty of 1.12
//! opcodes were gone by 1.14).
//!
//! ```ignore
//! pub const CMSG_PING: Opcode = Opcode { vanilla: 0x01DC, modern: 0x3768 };
//! pub const CMSG_BATTLE_PAY_GET_PRODUCT_LIST: Opcode = Opcode { modern: 0x36C2, ..Opcode::NONE };
//! ```
//!
//! Adding a third protocol later means adding a field and a `..Opcode::NONE` default — existing
//! constants do not change.
//!
//! ## Constructing these by hand is deliberately impossible
//!
//! There is no `Opcode::new`, no `From<u16>`. A numerically-built `Opcode` would carry `0` in the
//! columns it did not know about and so compare unequal to every constant here — every `match` arm
//! in the dispatchers would silently fall through and the server would look deaf, with no compile
//! error and no panic. Inbound numbers are resolved with [`Opcode::from_vanilla_wire`] /
//! [`Opcode::from_modern_wire`], which return `None` for an unrecognised number.
//!
//! ## Which modern table
//!
//! The `modern` column is build **42597** (1.14.2), i.e. HermesProxy's `V2_5_3_41750` table. Builds
//! renumber: 580 of the 1723 opcodes shared between 40688 and 41750 have different values, so this
//! column is not portable to another build without regenerating it.

/// The client build the `modern` column was generated for.
///
/// Opcodes are renumbered between builds — 580 of the 1723 shared between 40688 and 41750 have
/// different values — so this table is only valid for this one. Serving a different build needs the
/// column regenerated (`tools/opcode-gen/opcode_gen.py`), not a tweak.
pub const MODERN_OPCODE_BUILD: u32 = 42597;

/// A packet opcode and its number on each protocol's wire.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opcode {
    vanilla: u32,
    modern: u32,
}

impl Opcode {
    /// The all-absent opcode, used as the `..` base so a constant only names the protocols it
    /// actually exists in.
    pub const NONE: Opcode = Opcode {
        vanilla: 0,
        modern: 0,
    };

    /// The vanilla wire number, or `0` if this opcode has no vanilla form.
    pub const fn vanilla(self) -> u32 {
        self.vanilla
    }

    /// The vanilla wire number as it appears in a server header.
    pub const fn vanilla_u16(self) -> u16 {
        self.vanilla as u16
    }

    /// The modern wire number, or `0` if this opcode has no modern form.
    pub const fn modern(self) -> u16 {
        self.modern as u16
    }

    /// Whether this opcode exists on the vanilla wire.
    pub const fn has_vanilla(self) -> bool {
        self.vanilla != 0
    }

    /// Whether this opcode exists on the modern wire.
    pub const fn has_modern(self) -> bool {
        self.modern != 0
    }

    /// Resolve a number read from a vanilla client header. `None` means we do not know it.
    pub fn from_vanilla_wire(value: u32) -> Option<Opcode> {
        lookup(&VANILLA_INDEX, VANILLA_LO, value)
    }

    /// Resolve a number read from a modern client header. `None` means we do not know it.
    pub fn from_modern_wire(value: u16) -> Option<Opcode> {
        lookup(&MODERN_INDEX, MODERN_LO, value as u32)
    }

    /// This opcode's constant name, for logging.
    pub fn name(self) -> Option<&'static str> {
        ALL.iter()
            .find(|(opcode, _)| *opcode == self)
            .map(|(_, name)| *name)
    }
}

/// The opcode table itself.
///
/// `rustfmt` is off here on purpose: one line per opcode keeps 577 constants greppable and makes
/// diffs readable. Reformatted, each becomes four lines and the table becomes unreviewable.
#[rustfmt::skip]
impl Opcode {
    // ============================================================================
    // Authentication & Connection
    // ============================================================================

    pub const CMSG_NULL_ACTION: Opcode = Opcode { vanilla: 0x000, ..Opcode::NONE };
    pub const CMSG_PING: Opcode = Opcode { vanilla: 0x01DC, modern: 0x3768 }; // 476
    pub const CMSG_AUTH_SESSION: Opcode = Opcode { vanilla: 0x01ED, modern: 0x3765 };
    pub const SMSG_AUTH_CHALLENGE: Opcode = Opcode { vanilla: 0x01EC, modern: 0x3048 };
    pub const SMSG_AUTH_RESPONSE: Opcode = Opcode { vanilla: 0x01EE, modern: 0x256D };
    pub const SMSG_PONG: Opcode = Opcode { vanilla: 0x001D, modern: 0x304E };

    // ============================================================================
    // Character Management
    // ============================================================================

    pub const CMSG_CHAR_CREATE: Opcode = Opcode { vanilla: 0x0036, modern: 0x3645 };
    pub const CMSG_CHAR_ENUM: Opcode = Opcode { vanilla: 0x0037, modern: 0x35E9 };
    pub const CMSG_CHAR_DELETE: Opcode = Opcode { vanilla: 0x0038, modern: 0x369C };
    pub const CMSG_PLAYER_LOGIN: Opcode = Opcode { vanilla: 0x003D, modern: 0x35EB };
    pub const CMSG_CHAR_RENAME: Opcode = Opcode { vanilla: 0x02C7, modern: 0x36C7 }; // 711
    pub const SMSG_CHAR_CREATE: Opcode = Opcode { vanilla: 0x003A, modern: 0x26F8 };
    pub const SMSG_CHAR_ENUM: Opcode = Opcode { vanilla: 0x003B, modern: 0x2583 };
    pub const SMSG_CHAR_DELETE: Opcode = Opcode { vanilla: 0x003C, modern: 0x26F9 };
    pub const SMSG_CHAR_RENAME: Opcode = Opcode { vanilla: 0x02C8, modern: 0x275E }; // 712
    pub const SMSG_CHARACTER_LOGIN_FAILED: Opcode = Opcode { vanilla: 0x0041, modern: 0x26FC }; // 65

    // ============================================================================
    // Logout
    // ============================================================================

    pub const CMSG_LOGOUT_REQUEST: Opcode = Opcode { vanilla: 0x004B, modern: 0x34D6 };
    pub const CMSG_LOGOUT_CANCEL: Opcode = Opcode { vanilla: 0x004E, modern: 0x34D8 };
    pub const SMSG_LOGOUT_RESPONSE: Opcode = Opcode { vanilla: 0x004C, modern: 0x267C };
    pub const SMSG_LOGOUT_COMPLETE: Opcode = Opcode { vanilla: 0x004D, modern: 0x267D };
    pub const SMSG_LOGOUT_CANCEL_ACK: Opcode = Opcode { vanilla: 0x004F, modern: 0x267E };

    // ============================================================================
    // World Entry & Time
    // ============================================================================

    pub const SMSG_NEW_WORLD: Opcode = Opcode { vanilla: 0x003E, modern: 0x2599 }; // 62
    pub const SMSG_TRANSFER_PENDING: Opcode = Opcode { vanilla: 0x003F, modern: 0x25C9 }; // 63
    pub const SMSG_LOGIN_SETTIMESPEED: Opcode = Opcode { vanilla: 0x0042, modern: 0x2704 }; // 66
    pub const SMSG_LOGIN_VERIFY_WORLD: Opcode = Opcode { vanilla: 0x0236, modern: 0x259A }; // 566
    pub const CMSG_QUERY_TIME: Opcode = Opcode { vanilla: 0x01CE, modern: 0x34D5 }; // 462
    pub const SMSG_QUERY_TIME_RESPONSE: Opcode = Opcode { vanilla: 0x01CF, modern: 0x26DC }; // 463

    // ============================================================================
    // Tutorial & Account Data
    // ============================================================================

    pub const SMSG_TUTORIAL_FLAGS: Opcode = Opcode { vanilla: 0x00FD, modern: 0x27B6 }; // 253
    pub const CMSG_TUTORIAL_FLAG: Opcode = Opcode { vanilla: 0x00FE, modern: 0x36E2 }; // 254
    pub const CMSG_TUTORIAL_CLEAR: Opcode = Opcode { vanilla: 0x00FF, ..Opcode::NONE }; // 255
    pub const CMSG_TUTORIAL_RESET: Opcode = Opcode { vanilla: 0x0100, ..Opcode::NONE }; // 256
    pub const CMSG_UPDATE_ACCOUNT_DATA: Opcode = Opcode { vanilla: 0x020B, modern: 0x3695 }; // 523
    pub const SMSG_UPDATE_ACCOUNT_DATA: Opcode = Opcode { vanilla: 0x020C, modern: 0x2700 }; // 524
    pub const CMSG_REQUEST_ACCOUNT_DATA: Opcode = Opcode { vanilla: 0x020A, modern: 0x3694 }; // 522
    pub const SMSG_UPDATE_ACCOUNT_DATA_COMPLETE: Opcode = Opcode { vanilla: 0x020D, ..Opcode::NONE }; // 525
    pub const SMSG_ACCOUNT_DATA_MD5: Opcode = Opcode { vanilla: 0x0209, modern: 0x2701 }; // 521
    pub const SMSG_ACCOUNT_DATA_TIMES: Opcode = Opcode { vanilla: 0x0209, modern: 0x2701 }; // 521

    // ============================================================================
    // Query Responses
    // ============================================================================

    pub const CMSG_NAME_QUERY: Opcode = Opcode { vanilla: 0x050, modern: 0x376F }; // CMSG_QUERY_PLAYER_NAME
    pub const SMSG_NAME_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x0051, modern: 0x3002 }; // 81
    pub const CMSG_CREATURE_QUERY: Opcode = Opcode { vanilla: 0x0060, modern: 0x326B }; // 96
    pub const SMSG_CREATURE_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x0061, modern: 0x2914 }; // 97
    pub const CMSG_ITEM_QUERY_SINGLE: Opcode = Opcode { vanilla: 0x056, ..Opcode::NONE };
    pub const CMSG_ITEM_QUERY_MULTIPLE: Opcode = Opcode { vanilla: 0x057, ..Opcode::NONE };
    pub const SMSG_ITEM_QUERY_SINGLE_RESPONSE: Opcode = Opcode { vanilla: 0x058, ..Opcode::NONE }; // 88
    pub const SMSG_ITEM_QUERY_MULTIPLE_RESPONSE: Opcode = Opcode { vanilla: 0x057, ..Opcode::NONE };
    pub const CMSG_GAMEOBJECT_QUERY: Opcode = Opcode { vanilla: 0x005E, modern: 0x326C }; // 94
    pub const SMSG_GAMEOBJECT_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x005F, modern: 0x2915 }; // 95
    pub const CMSG_PAGE_TEXT_QUERY: Opcode = Opcode { vanilla: 0x005A, modern: 0x326F }; // 90
    pub const SMSG_PAGE_TEXT_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x005B, modern: 0x2917 }; // 91
    pub const CMSG_ITEM_TEXT_QUERY: Opcode = Opcode { vanilla: 0x0243, modern: 0x32BD }; // 579
    pub const SMSG_ITEM_TEXT_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x0244, modern: 0x291E }; // 580
    pub const MSG_CORPSE_QUERY: Opcode = Opcode { vanilla: 0x0216, ..Opcode::NONE }; // 534
    pub const CMSG_ITEM_NAME_QUERY: Opcode = Opcode { vanilla: 0x02C4, ..Opcode::NONE }; // 708
    pub const SMSG_ITEM_NAME_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x02C5, ..Opcode::NONE }; // 709

    // ============================================================================
    // Object Updates
    // ============================================================================

    pub const SMSG_UPDATE_OBJECT: Opcode = Opcode { vanilla: 0x00A9, modern: 0x27C4 }; // 169
    pub const SMSG_COMPRESSED_UPDATE_OBJECT: Opcode = Opcode { vanilla: 0x01F6, ..Opcode::NONE }; // 502
    pub const SMSG_COMPRESSED_MOVES: Opcode = Opcode { vanilla: 0x02B3, ..Opcode::NONE }; // 691
    pub const SMSG_DESTROY_OBJECT: Opcode = Opcode { vanilla: 0x00AA, ..Opcode::NONE }; // 170

    // ============================================================================
    // Movement - Basic
    // ============================================================================

    pub const MSG_MOVE_HEARTBEAT: Opcode = Opcode { vanilla: 0x00EE, modern: 0x3A0F }; // 238
    pub const MSG_MOVE_START_FORWARD: Opcode = Opcode { vanilla: 0x00B5, modern: 0x39E4 }; // 181
    pub const MSG_MOVE_START_BACKWARD: Opcode = Opcode { vanilla: 0x00B6, modern: 0x39E5 }; // 182
    pub const MSG_MOVE_STOP: Opcode = Opcode { vanilla: 0x00B7, modern: 0x39E6 }; // 183
    pub const MSG_MOVE_START_STRAFE_LEFT: Opcode = Opcode { vanilla: 0x00B8, modern: 0x39E7 }; // 184
    pub const MSG_MOVE_START_STRAFE_RIGHT: Opcode = Opcode { vanilla: 0x00B9, modern: 0x39E8 }; // 185
    pub const MSG_MOVE_STOP_STRAFE: Opcode = Opcode { vanilla: 0x00BA, modern: 0x39E9 }; // 186
    pub const MSG_MOVE_JUMP: Opcode = Opcode { vanilla: 0x00BB, modern: 0x39EA }; // 187
    pub const MSG_MOVE_START_TURN_LEFT: Opcode = Opcode { vanilla: 0x00BC, modern: 0x39EC }; // 188
    pub const MSG_MOVE_START_TURN_RIGHT: Opcode = Opcode { vanilla: 0x00BD, modern: 0x39ED }; // 189
    pub const MSG_MOVE_STOP_TURN: Opcode = Opcode { vanilla: 0x00BE, modern: 0x39EE }; // 190
    pub const MSG_MOVE_SET_FACING: Opcode = Opcode { vanilla: 0x00DA, modern: 0x3A07 }; // 218
    pub const MSG_MOVE_SET_PITCH: Opcode = Opcode { vanilla: 0x00DB, modern: 0x3A08 }; // 219
    pub const MSG_MOVE_WORLDPORT_ACK: Opcode = Opcode { vanilla: 0x00DC, ..Opcode::NONE }; // 220
    pub const MSG_MOVE_FALL_LAND: Opcode = Opcode { vanilla: 0x00C9, modern: 0x39F9 }; // 201

    // ============================================================================
    // Movement - Advanced
    // ============================================================================

    pub const CMSG_SET_ACTIVE_MOVER: Opcode = Opcode { vanilla: 0x026A, modern: 0x3A3B }; // 618
    pub const CMSG_MOVE_SPLINE_DONE: Opcode = Opcode { vanilla: 0x02C9, modern: 0x3A17 }; // 713
    pub const CMSG_MOVE_FALL_RESET: Opcode = Opcode { vanilla: 0x02CA, modern: 0x3A18 }; // 714
    pub const CMSG_MOVE_TIME_SKIPPED: Opcode = Opcode { vanilla: 0x02CE, modern: 0x3A1A }; // 718
    pub const CMSG_MOVE_FEATHER_FALL_ACK: Opcode = Opcode { vanilla: 0x02CF, modern: 0x3A1B }; // 719
    pub const CMSG_MOVE_WATER_WALK_ACK: Opcode = Opcode { vanilla: 0x02D0, modern: 0x3A1C }; // 720
    pub const CMSG_MOVE_NOT_ACTIVE_MOVER: Opcode = Opcode { vanilla: 0x02D1, ..Opcode::NONE }; // 721
    pub const MSG_MOVE_TELEPORT_ACK: Opcode = Opcode { vanilla: 0x00C7, ..Opcode::NONE }; // 199
    pub const MSG_MOVE_TELEPORT: Opcode = Opcode { vanilla: 0x00C5, ..Opcode::NONE }; // 197
    pub const MSG_MOVE_KNOCK_BACK: Opcode = Opcode { vanilla: 0x00F1, ..Opcode::NONE }; // 241
    pub const MSG_MOVE_TIME_SKIPPED: Opcode = Opcode { vanilla: 0x02CE, modern: 0x3A1A }; // 718 (same wire value as CMSG, used for observer rebroadcast)
    pub const CMSG_MOUNTSPECIAL_ANIM: Opcode = Opcode { vanilla: 0x0171, modern: 0x327B }; // 369
    pub const SMSG_MOUNTSPECIAL_ANIM: Opcode = Opcode { vanilla: 0x0172, modern: 0x2698 }; // 370

    // ============================================================================
    // Movement - Speed Changes (Force - to controller)
    // ============================================================================

    pub const SMSG_FORCE_WALK_SPEED_CHANGE: Opcode = Opcode { vanilla: 0x02DA, ..Opcode::NONE }; // 730
    pub const SMSG_FORCE_RUN_SPEED_CHANGE: Opcode = Opcode { vanilla: 0x00E2, ..Opcode::NONE }; // 226
    pub const SMSG_FORCE_RUN_BACK_SPEED_CHANGE: Opcode = Opcode { vanilla: 0x00E4, ..Opcode::NONE }; // 228
    pub const SMSG_FORCE_SWIM_SPEED_CHANGE: Opcode = Opcode { vanilla: 0x00E6, ..Opcode::NONE }; // 230
    pub const SMSG_FORCE_SWIM_BACK_SPEED_CHANGE: Opcode = Opcode { vanilla: 0x02DC, ..Opcode::NONE }; // 732
    pub const SMSG_FORCE_TURN_RATE_CHANGE: Opcode = Opcode { vanilla: 0x02DE, ..Opcode::NONE }; // 734

    // Client acknowledgements of the forced speed changes above
    pub const CMSG_FORCE_WALK_SPEED_CHANGE_ACK: Opcode = Opcode { vanilla: 0x02DB, modern: 0x3A20 }; // 731
    pub const CMSG_FORCE_RUN_SPEED_CHANGE_ACK: Opcode = Opcode { vanilla: 0x00E3, modern: 0x3A0A }; // 227
    pub const CMSG_FORCE_RUN_BACK_SPEED_CHANGE_ACK: Opcode = Opcode { vanilla: 0x00E5, modern: 0x3A0B }; // 229
    pub const CMSG_FORCE_SWIM_SPEED_CHANGE_ACK: Opcode = Opcode { vanilla: 0x00E7, modern: 0x3A0C }; // 231
    pub const CMSG_FORCE_SWIM_BACK_SPEED_CHANGE_ACK: Opcode = Opcode { vanilla: 0x02DD, modern: 0x3A21 }; // 733
    pub const CMSG_FORCE_TURN_RATE_CHANGE_ACK: Opcode = Opcode { vanilla: 0x02DF, modern: 0x3A22 }; // 735

    // ============================================================================
    // Movement - Speed Changes (Spline - server-controlled units)
    // ============================================================================

    pub const SMSG_SPLINE_SET_WALK_SPEED: Opcode = Opcode { vanilla: 0x0301, modern: 0x2DED }; // 769
    pub const SMSG_SPLINE_SET_RUN_SPEED: Opcode = Opcode { vanilla: 0x02FE, modern: 0x2DE7 }; // 766
    pub const SMSG_SPLINE_SET_RUN_BACK_SPEED: Opcode = Opcode { vanilla: 0x02FF, modern: 0x2DE8 }; // 767
    pub const SMSG_SPLINE_SET_SWIM_SPEED: Opcode = Opcode { vanilla: 0x0300, modern: 0x2DE9 }; // 768
    pub const SMSG_SPLINE_SET_SWIM_BACK_SPEED: Opcode = Opcode { vanilla: 0x0302, modern: 0x2DEA }; // 770
    pub const SMSG_SPLINE_SET_TURN_RATE: Opcode = Opcode { vanilla: 0x0303, modern: 0x2DEE }; // 771

    // ============================================================================
    // Movement - Speed Changes (MSG - to observers)
    // ============================================================================

    pub const MSG_MOVE_SET_WALK_SPEED: Opcode = Opcode { vanilla: 0x00D1, ..Opcode::NONE }; // 209
    pub const MSG_MOVE_SET_RUN_SPEED: Opcode = Opcode { vanilla: 0x00CD, ..Opcode::NONE }; // 205
    pub const MSG_MOVE_SET_RUN_BACK_SPEED: Opcode = Opcode { vanilla: 0x00CF, ..Opcode::NONE }; // 207
    pub const MSG_MOVE_SET_SWIM_SPEED: Opcode = Opcode { vanilla: 0x00D3, ..Opcode::NONE }; // 211
    pub const MSG_MOVE_SET_SWIM_BACK_SPEED: Opcode = Opcode { vanilla: 0x00D5, ..Opcode::NONE }; // 213
    pub const MSG_MOVE_SET_TURN_RATE: Opcode = Opcode { vanilla: 0x00D8, ..Opcode::NONE }; // 216

    // ============================================================================
    // Movement - Flags (Force - to controller)
    // ============================================================================

    pub const SMSG_FORCE_MOVE_ROOT: Opcode = Opcode { vanilla: 0x00E8, modern: 0x2DF9 }; // 232
    pub const CMSG_FORCE_MOVE_ROOT_ACK: Opcode = Opcode { vanilla: 0x00E9, modern: 0x3A0D }; // 233
    pub const SMSG_FORCE_MOVE_UNROOT: Opcode = Opcode { vanilla: 0x00EA, modern: 0x2DFA }; // 234
    pub const SMSG_MOVE_WATER_WALK: Opcode = Opcode { vanilla: 0x00DE, modern: 0x2DFB }; // 222
    pub const SMSG_MOVE_LAND_WALK: Opcode = Opcode { vanilla: 0x00DF, modern: 0x2DFE }; // 223
    pub const SMSG_MOVE_SET_HOVER: Opcode = Opcode { vanilla: 0x00F4, modern: 0x2E01 }; // 244
    pub const SMSG_MOVE_UNSET_HOVER: Opcode = Opcode { vanilla: 0x00F5, modern: 0x2E02 }; // 245
    pub const SMSG_MOVE_FEATHER_FALL: Opcode = Opcode { vanilla: 0x00F2, modern: 0x2DFF }; // 242
    pub const SMSG_MOVE_NORMAL_FALL: Opcode = Opcode { vanilla: 0x00F3, modern: 0x2E00 }; // 243
    pub const SMSG_MOVE_KNOCK_BACK: Opcode = Opcode { vanilla: 0x00EF, modern: 0x2E03 }; // 239
    pub const CMSG_MOVE_KNOCK_BACK_ACK: Opcode = Opcode { vanilla: 0x00F0, modern: 0x3A11 }; // 240

    // ============================================================================
    // Movement - Flags (Spline - server-controlled units)
    // ============================================================================

    pub const SMSG_SPLINE_MOVE_ROOT: Opcode = Opcode { vanilla: 0x031A, modern: 0x2E19 }; // 794
    pub const SMSG_SPLINE_MOVE_UNROOT: Opcode = Opcode { vanilla: 0x0304, modern: 0x2E1A }; // 772
    pub const SMSG_SPLINE_MOVE_WATER_WALK: Opcode = Opcode { vanilla: 0x0309, modern: 0x2E23 }; // 777
    pub const SMSG_SPLINE_MOVE_LAND_WALK: Opcode = Opcode { vanilla: 0x030A, modern: 0x2E24 }; // 778
    pub const SMSG_SPLINE_MOVE_SET_HOVER: Opcode = Opcode { vanilla: 0x0307, modern: 0x2E21 }; // 775
    pub const SMSG_SPLINE_MOVE_UNSET_HOVER: Opcode = Opcode { vanilla: 0x0308, modern: 0x2E22 }; // 776
    pub const SMSG_SPLINE_MOVE_FEATHER_FALL: Opcode = Opcode { vanilla: 0x0305, modern: 0x2E1F }; // 773
    pub const SMSG_SPLINE_MOVE_NORMAL_FALL: Opcode = Opcode { vanilla: 0x0306, modern: 0x2E20 }; // 774
    pub const SMSG_SPLINE_MOVE_SET_RUN_MODE: Opcode = Opcode { vanilla: 0x030D, modern: 0x2E27 }; // 781
    pub const SMSG_SPLINE_MOVE_SET_WALK_MODE: Opcode = Opcode { vanilla: 0x030E, modern: 0x2E28 }; // 782

    // ============================================================================
    // Movement - Flags (MSG - to observers)
    // ============================================================================

    pub const MSG_MOVE_ROOT: Opcode = Opcode { vanilla: 0x00EC, ..Opcode::NONE }; // 236
    pub const MSG_MOVE_UNROOT: Opcode = Opcode { vanilla: 0x00ED, ..Opcode::NONE }; // 237
    pub const MSG_MOVE_WATER_WALK: Opcode = Opcode { vanilla: 0x02B1, ..Opcode::NONE }; // 689
    pub const MSG_MOVE_HOVER: Opcode = Opcode { vanilla: 0x00F7, ..Opcode::NONE }; // 247
    pub const MSG_MOVE_FEATHER_FALL: Opcode = Opcode { vanilla: 0x02B0, ..Opcode::NONE }; // 688

    // ============================================================================
    // Monster Movement
    // ============================================================================

    pub const SMSG_MONSTER_MOVE: Opcode = Opcode { vanilla: 0x00DD, modern: 0x2DD4 }; // 221
    pub const SMSG_MONSTER_MOVE_TRANSPORT: Opcode = Opcode { vanilla: 0x02AE, ..Opcode::NONE }; // 686

    // ============================================================================
    // Combat
    // ============================================================================

    pub const CMSG_ATTACKSWING: Opcode = Opcode { vanilla: 0x0141, modern: 0x3250 }; // 321
    pub const CMSG_ATTACKSTOP: Opcode = Opcode { vanilla: 0x0142, modern: 0x3251 }; // 322
    pub const SMSG_ATTACKSTART: Opcode = Opcode { vanilla: 0x0143, modern: 0x293B }; // 323
    pub const SMSG_ATTACKSTOP: Opcode = Opcode { vanilla: 0x0144, modern: 0x293C }; // 324
    pub const SMSG_ATTACKSWING_NOTINRANGE: Opcode = Opcode { vanilla: 0x0145, ..Opcode::NONE }; // 325
    pub const SMSG_ATTACKSWING_BADFACING: Opcode = Opcode { vanilla: 0x0146, ..Opcode::NONE }; // 326
    pub const SMSG_ATTACKSWING_NOTSTANDING: Opcode = Opcode { vanilla: 0x0147, ..Opcode::NONE }; // 327
    pub const SMSG_ATTACKSWING_DEADTARGET: Opcode = Opcode { vanilla: 0x0148, ..Opcode::NONE }; // 328
    pub const SMSG_ATTACKSWING_CANT_ATTACK: Opcode = Opcode { vanilla: 0x0149, ..Opcode::NONE }; // 329
    pub const SMSG_ATTACKERSTATEUPDATE: Opcode = Opcode { vanilla: 0x014A, modern: 0x2950 }; // 330

    // ============================================================================
    // Selection & Targeting
    // ============================================================================

    pub const CMSG_SET_SELECTION: Opcode = Opcode { vanilla: 0x013D, modern: 0x3527 }; // 317

    // ============================================================================
    // Stand State
    // ============================================================================

    pub const CMSG_STANDSTATECHANGE: Opcode = Opcode { vanilla: 0x0101, modern: 0x318D }; // 257
    pub const SMSG_STANDSTATE_UPDATE: Opcode = Opcode { vanilla: 0x029D, modern: 0x2713 }; // 669

    // ============================================================================
    // Spell Casting
    // ============================================================================

    pub const CMSG_CAST_SPELL: Opcode = Opcode { vanilla: 0x012E, modern: 0x3294 }; // 302
    pub const CMSG_CANCEL_CAST: Opcode = Opcode { vanilla: 0x012F, modern: 0x3297 }; // 303
    pub const CMSG_CANCEL_AURA: Opcode = Opcode { vanilla: 0x0136, modern: 0x31B0 }; // 310
    pub const CMSG_CANCEL_GROWTH_AURA: Opcode = Opcode { vanilla: 0x029B, modern: 0x326A }; // 667
    pub const CMSG_CANCEL_AUTO_REPEAT_SPELL: Opcode = Opcode { vanilla: 0x026D, modern: 0x34E7 }; // 621
    pub const SMSG_CANCEL_AUTO_REPEAT: Opcode = Opcode { vanilla: 0x029C, modern: 0x26D6 }; // 668
    pub const CMSG_CANCEL_CHANNELING: Opcode = Opcode { vanilla: 0x013B, modern: 0x3265 }; // 315
    pub const CMSG_CANCEL_CHANNELLING: Opcode = Opcode { vanilla: 0x013B, modern: 0x3265 }; // 315 (alias)
    pub const CMSG_USE_ITEM: Opcode = Opcode { vanilla: 0x00AB, modern: 0x3290 }; // 171
    pub const CMSG_NEW_SPELL_SLOT: Opcode = Opcode { vanilla: 0x012D, ..Opcode::NONE }; // 301
    pub const SMSG_SPELL_START: Opcode = Opcode { vanilla: 0x0131, modern: 0x2C3A }; // 305
    pub const SMSG_SPELL_GO: Opcode = Opcode { vanilla: 0x0132, modern: 0x2C39 }; // 306
    pub const SMSG_CAST_RESULT: Opcode = Opcode { vanilla: 0x0130, modern: 0x2C57 }; // 304
    pub const SMSG_SPELL_COOLDOWN: Opcode = Opcode { vanilla: 0x0134, modern: 0x2C15 }; // 308
    pub const MSG_CHANNEL_START: Opcode = Opcode { vanilla: 0x0139, ..Opcode::NONE }; // 313
    pub const MSG_CHANNEL_UPDATE: Opcode = Opcode { vanilla: 0x013A, modern: 0x2C35 }; // 1.14 calls it SMSG_SPELL_CHANNEL_UPDATE // 314
    pub const SMSG_SPELL_INTERRUPTED: Opcode = Opcode { vanilla: 0x0152, ..Opcode::NONE }; // 338
    pub const SMSG_SPELL_DELAYED: Opcode = Opcode { vanilla: 0x01E2, modern: 0x2C3F }; // 482
    pub const SMSG_SPELL_FAILED_OTHER: Opcode = Opcode { vanilla: 0x02A6, modern: 0x2C55 }; // 678
    pub const SMSG_SPELL_UPDATE_CHAIN_TARGETS: Opcode = Opcode { vanilla: 0x0330, ..Opcode::NONE }; // 816
    pub const SMSG_SET_PROFICIENCY: Opcode = Opcode { vanilla: 0x0127, modern: 0x272C }; // 295
    pub const SMSG_INITIAL_SPELLS: Opcode = Opcode { vanilla: 0x012A, modern: 0x2C2A }; // 298
    pub const SMSG_LEARNED_SPELL: Opcode = Opcode { vanilla: 0x012B, modern: 0x2C4D }; // 1.14 calls it SMSG_LEARNED_SPELLS // 299
    pub const SMSG_REMOVED_SPELL: Opcode = Opcode { vanilla: 0x0203, modern: 0x2C4E }; // 515
    pub const SMSG_SPELL_FAILURE: Opcode = Opcode { vanilla: 0x0133, modern: 0x2C53 }; // 307
    pub const SMSG_CLEAR_COOLDOWN: Opcode = Opcode { vanilla: 0x01DE, modern: 0x26B3 }; // 478

    // ============================================================================
    // Auras
    // ============================================================================

    pub const SMSG_AURA_UPDATE: Opcode = Opcode { vanilla: 0x0495, modern: 0x2C22 }; // 1173
    pub const SMSG_AURA_UPDATE_ALL: Opcode = Opcode { vanilla: 0x0496, ..Opcode::NONE }; // 1174
    pub const SMSG_UPDATE_AURA_DURATION: Opcode = Opcode { vanilla: 0x0137, ..Opcode::NONE }; // 311
    pub const SMSG_SET_EXTRA_AURA_INFO: Opcode = Opcode { vanilla: 0x04A7, ..Opcode::NONE }; // 1191
    pub const SMSG_PERIODICAURALOG: Opcode = Opcode { vanilla: 0x024E, modern: 0x2C1B }; // 590

    // ============================================================================
    // Combat Log
    // ============================================================================

    pub const SMSG_SPELLDAMAGELOG: Opcode = Opcode { vanilla: 0x014E, modern: 0x2949 }; // 334
    pub const SMSG_SPELLHEALLOG: Opcode = Opcode { vanilla: 0x0150, modern: 0x2C1D }; // 336
    pub const SMSG_SPELLLOGMISS: Opcode = Opcode { vanilla: 0x014C, ..Opcode::NONE }; // 332
    pub const SMSG_SPELLENERGIZELOG: Opcode = Opcode { vanilla: 0x0151, modern: 0x2C1C }; // 337
    pub const SMSG_SPELLNONMELEEDAMAGELOG: Opcode = Opcode { vanilla: 0x0148, ..Opcode::NONE }; // 328
    pub const SMSG_SPELLLOGEXECUTE: Opcode = Opcode { vanilla: 0x024C, modern: 0x2C40 }; // 588
    pub const SMSG_SPELLINSTAKILLLOG: Opcode = Opcode { vanilla: 0x033F, ..Opcode::NONE }; // 815
    pub const SMSG_PROCRESIST: Opcode = Opcode { vanilla: 0x0260, modern: 0x2752 }; // 608
    pub const SMSG_SPELLORDAMAGE_IMMUNE: Opcode = Opcode { vanilla: 0x0263, modern: 0x2C2F }; // 611

    // ============================================================================
    // Action Bar
    // ============================================================================

    pub const CMSG_SET_ACTION_BUTTON: Opcode = Opcode { vanilla: 0x0128, modern: 0x3558 }; // 296
    pub const SMSG_ACTION_BUTTONS: Opcode = Opcode { vanilla: 0x0129, modern: 0x25D9 }; // 297

    // ============================================================================
    // Death & Resurrection
    // ============================================================================

    pub const SMSG_DURABILITY_DAMAGE_DEATH: Opcode = Opcode { vanilla: 0x02BD, modern: 0x273D }; // 701
    pub const SMSG_CORPSE_RECLAIM_DELAY: Opcode = Opcode { vanilla: 0x0269, modern: 0x2741 }; // 617
    pub const CMSG_REPOP_REQUEST: Opcode = Opcode { vanilla: 0x015A, modern: 0x3525 }; // 346
    pub const CMSG_RESURRECT_RESPONSE: Opcode = Opcode { vanilla: 0x015C, modern: 0x3685 }; // 348
    pub const CMSG_RECLAIM_CORPSE: Opcode = Opcode { vanilla: 0x01D2, modern: 0x34DB }; // 466
    pub const SMSG_RESURRECT_REQUEST: Opcode = Opcode { vanilla: 0x015B, modern: 0x257E }; // 347
    pub const SMSG_SPIRIT_HEALER_CONFIRM: Opcode = Opcode { vanilla: 0x0222, modern: 0x270C }; // 546
    pub const CMSG_SPIRIT_HEALER_ACTIVATE: Opcode = Opcode { vanilla: 0x021C, modern: 0x34AF }; // 540
    pub const CMSG_SELF_RES: Opcode = Opcode { vanilla: 0x02B3, modern: 0x3530 }; // 691

    // ============================================================================
    // NPC Interaction - Gossip
    // ============================================================================

    pub const CMSG_GOSSIP_HELLO: Opcode = Opcode { vanilla: 0x017B, modern: 0x3492 }; // 379
    pub const CMSG_GOSSIP_SELECT_OPTION: Opcode = Opcode { vanilla: 0x017C, modern: 0x3494 }; // 380
    pub const SMSG_GOSSIP_MESSAGE: Opcode = Opcode { vanilla: 0x017D, modern: 0x2A98 }; // 381
    pub const SMSG_GOSSIP_COMPLETE: Opcode = Opcode { vanilla: 0x017E, modern: 0x2A97 }; // 382
    pub const SMSG_GOSSIP_POI: Opcode = Opcode { vanilla: 0x0223, modern: 0x2790 }; // 547
    pub const SMSG_NPC_TEXT_UPDATE: Opcode = Opcode { vanilla: 0x0180, modern: 0x2916 }; // 384
    pub const CMSG_NPC_TEXT_QUERY: Opcode = Opcode { vanilla: 0x017F, modern: 0x326D }; // 383

    // ============================================================================
    // NPC Interaction - Vendor
    // ============================================================================

    pub const CMSG_LIST_INVENTORY: Opcode = Opcode { vanilla: 0x019E, modern: 0x34A1 }; // 414
    pub const SMSG_LIST_INVENTORY: Opcode = Opcode { vanilla: 0x019F, modern: 0x25B5 }; // 415
    pub const CMSG_SELL_ITEM: Opcode = Opcode { vanilla: 0x01A0, modern: 0x34A2 }; // 416
    pub const SMSG_SELL_ITEM: Opcode = Opcode { vanilla: 0x01A1, modern: 0x26BD }; // 417
    pub const CMSG_BUY_ITEM: Opcode = Opcode { vanilla: 0x01A2, modern: 0x34A3 }; // 418
    pub const CMSG_BUY_ITEM_IN_SLOT: Opcode = Opcode { vanilla: 0x01A3, ..Opcode::NONE }; // 419
    pub const SMSG_BUY_ITEM: Opcode = Opcode { vanilla: 0x01A4, modern: 0x26BE }; // 420
    pub const SMSG_BUY_FAILED: Opcode = Opcode { vanilla: 0x01A5, modern: 0x26BF }; // 421
    pub const SMSG_ITEM_PUSH_RESULT: Opcode = Opcode { vanilla: 0x0166, modern: 0x261C }; // 358
    pub const CMSG_BUYBACK_ITEM: Opcode = Opcode { vanilla: 0x0290, modern: 0x34A4 }; // 656

    // ============================================================================
    // NPC Interaction - Trainer
    // ============================================================================

    pub const CMSG_TRAINER_LIST: Opcode = Opcode { vanilla: 0x01B0, modern: 0x34AD }; // 432
    pub const SMSG_TRAINER_LIST: Opcode = Opcode { vanilla: 0x01B1, modern: 0x26D7 }; // 433
    pub const CMSG_TRAINER_BUY_SPELL: Opcode = Opcode { vanilla: 0x01B2, modern: 0x34AE }; // 434
    pub const SMSG_TRAINER_BUY_SUCCEEDED: Opcode = Opcode { vanilla: 0x01B3, ..Opcode::NONE }; // 435
    pub const SMSG_TRAINER_BUY_FAILED: Opcode = Opcode { vanilla: 0x01B4, modern: 0x26D8 }; // 436

    // ============================================================================
    // NPC Interaction - Banker
    // ============================================================================

    pub const CMSG_BANKER_ACTIVATE: Opcode = Opcode { vanilla: 0x01B5, modern: 0x34B3 }; // 439
    pub const SMSG_SHOW_BANK: Opcode = Opcode { vanilla: 0x01B8, modern: 0x2678 }; // 440
    pub const CMSG_BUY_BANK_SLOT: Opcode = Opcode { vanilla: 0x01B9, modern: 0x34B4 }; // 441
    pub const SMSG_BUY_BANK_SLOT_RESULT: Opcode = Opcode { vanilla: 0x0216, ..Opcode::NONE }; // 534
    pub const CMSG_AUTOBANK_ITEM: Opcode = Opcode { vanilla: 0x0283, modern: 0x3997 }; // 643
    pub const CMSG_AUTOSTORE_BANK_ITEM: Opcode = Opcode { vanilla: 0x0282, modern: 0x3996 }; // 642

    // ============================================================================
    // NPC Interaction - Other
    // ============================================================================

    pub const CMSG_BINDER_ACTIVATE: Opcode = Opcode { vanilla: 0x01B5, modern: 0x34B2 }; // 437
    pub const MSG_TABARDVENDOR_ACTIVATE: Opcode = Opcode { vanilla: 0x01F2, ..Opcode::NONE }; // 498

    // ============================================================================
    // Taxi
    // ============================================================================

    pub const CMSG_TAXINODE_STATUS_QUERY: Opcode = Opcode { vanilla: 0x01AA, modern: 0x34A8 }; // 426
    pub const SMSG_TAXINODE_STATUS: Opcode = Opcode { vanilla: 0x01AB, modern: 0x2675 }; // 427
    pub const CMSG_TAXIQUERYAVAILABLENODES: Opcode = Opcode { vanilla: 0x01AC, modern: 0x34AA }; // 428
    pub const SMSG_SHOWTAXINODES: Opcode = Opcode { vanilla: 0x01A9, modern: 0x26C5 }; // 425
    pub const CMSG_ACTIVATETAXI: Opcode = Opcode { vanilla: 0x01AD, modern: 0x34AB }; // 429
    pub const SMSG_ACTIVATETAXIREPLY: Opcode = Opcode { vanilla: 0x01AE, modern: 0x2676 }; // 430
    pub const SMSG_NEW_TAXI_PATH: Opcode = Opcode { vanilla: 0x01AF, modern: 0x2677 }; // 431

    // ============================================================================
    // Talents & Skills
    // ============================================================================

    pub const CMSG_LEARN_TALENT: Opcode = Opcode { vanilla: 0x0251, modern: 0x3551 }; // 593
    pub const CMSG_UNLEARN_TALENTS: Opcode = Opcode { vanilla: 0x0213, ..Opcode::NONE }; // 531
    pub const CMSG_UNLEARN_SPELL: Opcode = Opcode { vanilla: 0x0201, ..Opcode::NONE }; // 513
    pub const CMSG_UNLEARN_SKILL: Opcode = Opcode { vanilla: 0x0202, modern: 0x34E5 }; // 514

    // ============================================================================
    // Bind Point
    // ============================================================================

    pub const SMSG_BINDPOINTUPDATE: Opcode = Opcode { vanilla: 0x0155, modern: 0x257D }; // 341
    pub const SMSG_BINDZONEREPLY: Opcode = Opcode { vanilla: 0x0157, ..Opcode::NONE }; // 343
    pub const SMSG_PLAYERBOUND: Opcode = Opcode { vanilla: 0x0158, modern: 0x2FF8 }; // 344
    pub const CMSG_SETDEATHBINDPOINT: Opcode = Opcode { vanilla: 0x0154, ..Opcode::NONE }; // 340
    pub const CMSG_GETDEATHBINDZONE: Opcode = Opcode { vanilla: 0x0156, ..Opcode::NONE }; // 342

    // ============================================================================
    // Rest & XP
    // ============================================================================

    pub const SMSG_SET_REST_START: Opcode = Opcode { vanilla: 0x021E, ..Opcode::NONE }; // 542
    pub const SMSG_LOG_XPGAIN: Opcode = Opcode { vanilla: 0x01D0, modern: 0x26DD }; // 464
    pub const SMSG_LEVELUP_INFO: Opcode = Opcode { vanilla: 0x01D4, modern: 0x26E1 }; // 468

    // ============================================================================
    // Environment & Mirror Timers
    // ============================================================================

    pub const SMSG_START_MIRROR_TIMER: Opcode = Opcode { vanilla: 0x0C1D, modern: 0x2706 }; // 3101
    pub const SMSG_STOP_MIRROR_TIMER: Opcode = Opcode { vanilla: 0x0C1E, modern: 0x2708 }; // 3102
    pub const SMSG_ENVIRONMENTALDAMAGELOG: Opcode = Opcode { vanilla: 0x0C1F, ..Opcode::NONE }; // 3103
    pub const SMSG_EXPLORATION_EXPERIENCE: Opcode = Opcode { vanilla: 0x01F8, modern: 0x2756 }; // 504

    // ============================================================================
    // World States & Factions
    // ============================================================================

    pub const SMSG_INIT_WORLD_STATES: Opcode = Opcode { vanilla: 0x02C2, modern: 0x273E }; // 706
    pub const SMSG_INITIALIZE_FACTIONS: Opcode = Opcode { vanilla: 0x0122, modern: 0x271B }; // 290
    pub const SMSG_SET_FACTION_STANDING: Opcode = Opcode { vanilla: 0x0124, modern: 0x2723 }; // 292
    pub const SMSG_SET_FACTION_VISIBLE: Opcode = Opcode { vanilla: 0x0123, modern: 0x2721 }; // 291
    pub const SMSG_SET_FORCED_REACTIONS: Opcode = Opcode { vanilla: 0x02A5, modern: 0x2714 }; // 677
    pub const CMSG_SET_FACTION_ATWAR: Opcode = Opcode { vanilla: 0x0125, modern: 0x34DE }; // 293
    pub const CMSG_SET_FACTION_INACTIVE: Opcode = Opcode { vanilla: 0x0317, modern: 0x34E0 }; // 791

    // ============================================================================
    // Cinematic
    // ============================================================================

    pub const SMSG_TRIGGER_CINEMATIC: Opcode = Opcode { vanilla: 0x00FA, modern: 0x27C3 }; // 250
    pub const CMSG_NEXT_CINEMATIC_CAMERA: Opcode = Opcode { vanilla: 0x00FB, modern: 0x3543 }; // 251
    pub const CMSG_COMPLETE_CINEMATIC: Opcode = Opcode { vanilla: 0x00FC, modern: 0x3544 }; // 252
    pub const CMSG_SET_ACTION_BAR_TOGGLES: Opcode = Opcode { vanilla: 0x0568, modern: 0x3531 }; // 1384

    // ============================================================================
    // Zone
    // ============================================================================

    pub const CMSG_ZONEUPDATE: Opcode = Opcode { vanilla: 0x01F4, ..Opcode::NONE }; // 500

    // ============================================================================
    // Item Management
    // ============================================================================

    pub const CMSG_OPEN_ITEM: Opcode = Opcode { vanilla: 0x00AC, modern: 0x32BE }; // 172
    pub const CMSG_READ_ITEM: Opcode = Opcode { vanilla: 0x00AD, modern: 0x32BF }; // 173
    pub const SMSG_READ_ITEM_OK: Opcode = Opcode { vanilla: 0x00AE, modern: 0x2799 }; // 174
    pub const SMSG_READ_ITEM_FAILED: Opcode = Opcode { vanilla: 0x00AF, modern: 0x27A1 }; // 175
    pub const SMSG_ITEM_COOLDOWN: Opcode = Opcode { vanilla: 0x00B0, modern: 0x27C0 }; // 176
    pub const SMSG_INVENTORY_CHANGE_FAILURE: Opcode = Opcode { vanilla: 0x0112, modern: 0x2DA5 }; // 274
    pub const SMSG_OPEN_CONTAINER: Opcode = Opcode { vanilla: 0x0113, modern: 0x2DA6 }; // 275
    pub const CMSG_AUTOEQUIP_GROUND_ITEM: Opcode = Opcode { vanilla: 0x0106, ..Opcode::NONE }; // 262
    pub const CMSG_AUTOSTORE_GROUND_ITEM: Opcode = Opcode { vanilla: 0x0107, ..Opcode::NONE }; // 263
    pub const CMSG_AUTOSTORE_LOOT_ITEM: Opcode = Opcode { vanilla: 0x0108, ..Opcode::NONE }; // 264
    pub const CMSG_STORE_LOOT_IN_SLOT: Opcode = Opcode { vanilla: 0x0109, ..Opcode::NONE }; // 265
    pub const CMSG_AUTOEQUIP_ITEM: Opcode = Opcode { vanilla: 0x010A, modern: 0x3998 }; // 266
    pub const CMSG_AUTOSTORE_BAG_ITEM: Opcode = Opcode { vanilla: 0x010B, modern: 0x3999 }; // 267
    pub const CMSG_SWAP_ITEM: Opcode = Opcode { vanilla: 0x010C, modern: 0x399A }; // 268
    pub const CMSG_SWAP_INV_ITEM: Opcode = Opcode { vanilla: 0x010D, modern: 0x399B }; // 269
    pub const CMSG_SPLIT_ITEM: Opcode = Opcode { vanilla: 0x010E, modern: 0x399C }; // 270
    pub const CMSG_AUTOEQUIP_ITEM_SLOT: Opcode = Opcode { vanilla: 0x010F, modern: 0x399D }; // 271
    pub const CMSG_DROP_ITEM: Opcode = Opcode { vanilla: 0x0110, ..Opcode::NONE }; // 272
    pub const CMSG_DESTROYITEM: Opcode = Opcode { vanilla: 0x0111, modern: 0x328B }; // 273
    pub const CMSG_INSPECT: Opcode = Opcode { vanilla: 0x0114, modern: 0x3528 }; // 276
    pub const SMSG_INSPECT: Opcode = Opcode { vanilla: 0x0115, modern: 0x262A }; // 277
    pub const MSG_INSPECT_HONOR_STATS: Opcode = Opcode { vanilla: 0x02D6, ..Opcode::NONE }; // 726
    pub const CMSG_REPAIR_ITEM: Opcode = Opcode { vanilla: 0x02A8, modern: 0x34EC }; // 680
    pub const SMSG_ITEM_TIME_UPDATE: Opcode = Opcode { vanilla: 0x01EB, modern: 0x274B }; // 491
    pub const SMSG_ITEM_ENCHANT_TIME_UPDATE: Opcode = Opcode { vanilla: 0x01EC, modern: 0x274C }; // 492
    pub const CMSG_SET_AMMO: Opcode = Opcode { vanilla: 0x0268, modern: 0x3559 }; // 619
    pub const CMSG_WRAP_ITEM: Opcode = Opcode { vanilla: 0x01D3, modern: 0x3994 }; // 467

    // ============================================================================
    // Gameobject
    // ============================================================================

    pub const CMSG_GAMEOBJ_USE: Opcode = Opcode { vanilla: 0x00B1, modern: 0x34EE }; // 177

    // ============================================================================
    // Area Trigger
    // ============================================================================

    pub const CMSG_AREATRIGGER: Opcode = Opcode { vanilla: 0x00B4, modern: 0x31D7 }; // 180

    // ============================================================================
    // Chat
    // ============================================================================

    pub const CMSG_MESSAGECHAT: Opcode = Opcode { vanilla: 0x0095, ..Opcode::NONE }; // 149
    pub const SMSG_MESSAGECHAT: Opcode = Opcode { vanilla: 0x0096, modern: 0x2BAD }; // 150
    pub const CMSG_CHAT_IGNORED: Opcode = Opcode { vanilla: 0x0225, modern: 0x37CB }; // 549
    pub const SMSG_CHAT_WRONG_FACTION: Opcode = Opcode { vanilla: 0x0219, ..Opcode::NONE }; // 537
    pub const SMSG_CHAT_PLAYER_NOT_FOUND: Opcode = Opcode { vanilla: 0x02A9, modern: 0x2BB7 }; // 681
    pub const SMSG_CHAT_RESTRICTED: Opcode = Opcode { vanilla: 0x02FD, modern: 0x2BB3 }; // 765
    pub const SMSG_CHAT_PLAYER_AMBIGUOUS: Opcode = Opcode { vanilla: 0x032D, modern: 0x2BB0 }; // 813
    pub const CMSG_CHAT_FILTERED: Opcode = Opcode { vanilla: 0x0331, modern: 0x37CC }; // 817

    // ============================================================================
    // Emote
    // ============================================================================

    pub const CMSG_EMOTE: Opcode = Opcode { vanilla: 0x0102, modern: 0x3540 }; // 258
    pub const CMSG_TEXT_EMOTE: Opcode = Opcode { vanilla: 0x0104, modern: 0x3488 }; // 260
    pub const SMSG_TEXT_EMOTE: Opcode = Opcode { vanilla: 0x0105, modern: 0x2673 }; // 261
    pub const SMSG_EMOTE: Opcode = Opcode { vanilla: 0x0103, modern: 0x27C2 }; // 259
    pub const SMSG_PLAY_OBJECT_SOUND: Opcode = Opcode { vanilla: 0x0278, modern: 0x2765 }; // 632
    pub const SMSG_PLAY_SOUND: Opcode = Opcode { vanilla: 0x02D2, modern: 0x2763 }; // 722
    pub const SMSG_PLAY_SPELL_VISUAL: Opcode = Opcode { vanilla: 0x01F3, modern: 0x2C49 }; // 499
    // 0x2C49 is SMSG_PLAY_SPELL_VISUAL_KIT, not SMSG_PLAY_SPELL_VISUAL (0x2C45). The proxy
    // translates the legacy message into the *kit* one, because vanilla's payload is a visual
    // kit id. The name here is the vanilla one.

    // ============================================================================
    // Channel
    // ============================================================================

    pub const CMSG_JOIN_CHANNEL: Opcode = Opcode { vanilla: 0x0097, modern: 0x37C8 }; // 151
    pub const CMSG_LEAVE_CHANNEL: Opcode = Opcode { vanilla: 0x0098, modern: 0x37C9 }; // 152
    pub const SMSG_CHANNEL_NOTIFY: Opcode = Opcode { vanilla: 0x0099, modern: 0x2BC0 }; // 153
    pub const CMSG_CHANNEL_LIST: Opcode = Opcode { vanilla: 0x009A, modern: 0x37D5 }; // 154
    pub const SMSG_CHANNEL_LIST: Opcode = Opcode { vanilla: 0x009B, modern: 0x2BC3 }; // 155
    pub const CMSG_CHANNEL_PASSWORD: Opcode = Opcode { vanilla: 0x009C, modern: 0x37D7 }; // 156
    pub const CMSG_CHANNEL_SET_OWNER: Opcode = Opcode { vanilla: 0x009D, modern: 0x37D8 }; // 157
    pub const CMSG_CHANNEL_OWNER: Opcode = Opcode { vanilla: 0x009E, modern: 0x37D9 }; // 158
    pub const CMSG_CHANNEL_MODERATOR: Opcode = Opcode { vanilla: 0x009F, modern: 0x37DB }; // 159
    pub const CMSG_CHANNEL_UNMODERATOR: Opcode = Opcode { vanilla: 0x00A0, modern: 0x37DC }; // 160
    pub const CMSG_CHANNEL_MUTE: Opcode = Opcode { vanilla: 0x00A1, ..Opcode::NONE }; // 161
    pub const CMSG_CHANNEL_UNMUTE: Opcode = Opcode { vanilla: 0x00A2, ..Opcode::NONE }; // 162
    pub const CMSG_CHANNEL_INVITE: Opcode = Opcode { vanilla: 0x00A3, modern: 0x37DF }; // 163
    pub const CMSG_CHANNEL_KICK: Opcode = Opcode { vanilla: 0x00A4, modern: 0x37E0 }; // 164
    pub const CMSG_CHANNEL_BAN: Opcode = Opcode { vanilla: 0x00A5, modern: 0x37E1 }; // 165
    pub const CMSG_CHANNEL_UNBAN: Opcode = Opcode { vanilla: 0x00A6, modern: 0x37E2 }; // 166
    pub const CMSG_CHANNEL_ANNOUNCEMENTS: Opcode = Opcode { vanilla: 0x00A7, modern: 0x37E3 }; // 167
    pub const CMSG_CHANNEL_MODERATE: Opcode = Opcode { vanilla: 0x00A8, ..Opcode::NONE }; // 168

    // ============================================================================
    // Social - Who & Friends
    // ============================================================================

    pub const CMSG_WHO: Opcode = Opcode { vanilla: 0x0062, modern: 0x3683 }; // 98
    pub const SMSG_WHO: Opcode = Opcode { vanilla: 0x0063, modern: 0x2BAE }; // 99
    pub const CMSG_FRIEND_LIST: Opcode = Opcode { vanilla: 0x0066, ..Opcode::NONE }; // 102
    pub const SMSG_FRIEND_LIST: Opcode = Opcode { vanilla: 0x0067, ..Opcode::NONE }; // 103
    pub const SMSG_FRIEND_STATUS: Opcode = Opcode { vanilla: 0x0068, modern: 0x2784 }; // 104
    pub const CMSG_ADD_FRIEND: Opcode = Opcode { vanilla: 0x0069, modern: 0x36D6 }; // 105
    pub const CMSG_DEL_FRIEND: Opcode = Opcode { vanilla: 0x006A, modern: 0x36D7 }; // 106
    pub const SMSG_IGNORE_LIST: Opcode = Opcode { vanilla: 0x006B, ..Opcode::NONE }; // 107
    pub const CMSG_ADD_IGNORE: Opcode = Opcode { vanilla: 0x006C, modern: 0x36DA }; // 108
    pub const CMSG_DEL_IGNORE: Opcode = Opcode { vanilla: 0x006D, modern: 0x36DB }; // 109

    // ============================================================================
    // Group
    // ============================================================================

    pub const CMSG_GROUP_INVITE: Opcode = Opcode { vanilla: 0x006E, modern: 0x3603 }; // 110
    pub const SMSG_GROUP_INVITE: Opcode = Opcode { vanilla: 0x006F, modern: 0x25BA }; // 111
    pub const MSG_PARTY_LEAVE: Opcode = Opcode { vanilla: 0x0071, ..Opcode::NONE }; // 113
    pub const CMSG_GROUP_ACCEPT: Opcode = Opcode { vanilla: 0x0072, ..Opcode::NONE }; // 114
    pub const CMSG_GROUP_DECLINE: Opcode = Opcode { vanilla: 0x0073, ..Opcode::NONE }; // 115
    pub const SMSG_GROUP_DECLINE: Opcode = Opcode { vanilla: 0x0074, modern: 0x278B }; // 116
    pub const CMSG_GROUP_UNINVITE: Opcode = Opcode { vanilla: 0x0075, ..Opcode::NONE }; // 117
    pub const SMSG_GROUP_UNINVITE: Opcode = Opcode { vanilla: 0x0077, modern: 0x278C }; // 119
    pub const CMSG_GROUP_SET_LEADER: Opcode = Opcode { vanilla: 0x0078, modern: 0x364D }; // 120
    pub const SMSG_GROUP_SET_LEADER: Opcode = Opcode { vanilla: 0x0079, modern: 0x2626 }; // 121
    pub const CMSG_LOOT_METHOD: Opcode = Opcode { vanilla: 0x007A, modern: 0x364B }; // 122
    pub const CMSG_GROUP_DISBAND: Opcode = Opcode { vanilla: 0x007B, ..Opcode::NONE }; // 123
    pub const SMSG_GROUP_DESTROYED: Opcode = Opcode { vanilla: 0x007C, modern: 0x278D }; // 124
    pub const SMSG_GROUP_LIST: Opcode = Opcode { vanilla: 0x007D, ..Opcode::NONE }; // 125
    pub const SMSG_PARTY_MEMBER_STATS: Opcode = Opcode { vanilla: 0x007E, modern: 0x274F }; // 126
    pub const SMSG_PARTY_COMMAND_RESULT: Opcode = Opcode { vanilla: 0x007F, modern: 0x278F }; // 127
    pub const CMSG_GROUP_CHANGE_SUB_GROUP: Opcode = Opcode { vanilla: 0x027E, modern: 0x364F }; // 638
    pub const CMSG_GROUP_SWAP_SUB_GROUP: Opcode = Opcode { vanilla: 0x0280, modern: 0x3650 }; // 640
    pub const CMSG_GROUP_ASSISTANT_LEADER: Opcode = Opcode { vanilla: 0x028F, modern: 0x3652 }; // 655
    pub const CMSG_GROUP_RAID_CONVERT: Opcode = Opcode { vanilla: 0x028E, modern: 0x3651 }; // 654
    pub const CMSG_REQUEST_PARTY_MEMBER_STATS: Opcode = Opcode { vanilla: 0x027F, modern: 0x3656 }; // 639
    pub const SMSG_PARTY_MEMBER_STATS_FULL: Opcode = Opcode { vanilla: 0x02F2, modern: 0x2750 }; // 754
    pub const MSG_RAID_TARGET_UPDATE: Opcode = Opcode { vanilla: 0x0321, ..Opcode::NONE }; // 801
    pub const MSG_RAID_READY_CHECK: Opcode = Opcode { vanilla: 0x0322, ..Opcode::NONE }; // 802
    pub const MSG_MINIMAP_PING: Opcode = Opcode { vanilla: 0x01D5, ..Opcode::NONE }; // 469
    pub const MSG_RANDOM_ROLL: Opcode = Opcode { vanilla: 0x01FB, ..Opcode::NONE }; // 507

    // ============================================================================
    // Loot
    // ============================================================================

    pub const CMSG_LOOT: Opcode = Opcode { vanilla: 0x015D, modern: 0x320C }; // 349
    pub const CMSG_LOOT_MONEY: Opcode = Opcode { vanilla: 0x015E, modern: 0x320D }; // 350
    pub const CMSG_LOOT_RELEASE: Opcode = Opcode { vanilla: 0x015F, modern: 0x3210 }; // 351
    pub const SMSG_LOOT_RESPONSE: Opcode = Opcode { vanilla: 0x0160, modern: 0x260D }; // 352
    pub const SMSG_LOOT_RELEASE_RESPONSE: Opcode = Opcode { vanilla: 0x0161, modern: 0x2614 }; // 353
    pub const SMSG_LOOT_REMOVED: Opcode = Opcode { vanilla: 0x0162, modern: 0x260E }; // 354
    pub const SMSG_LOOT_MONEY_NOTIFY: Opcode = Opcode { vanilla: 0x0163, modern: 0x2615 }; // 355
    pub const SMSG_LOOT_CLEAR_MONEY: Opcode = Opcode { vanilla: 0x0165, ..Opcode::NONE }; // 357
    pub const CMSG_LOOT_ROLL: Opcode = Opcode { vanilla: 0x02A0, modern: 0x3211 }; // 672
    pub const SMSG_LOOT_START_ROLL: Opcode = Opcode { vanilla: 0x02A1, modern: 0x2616 }; // 673
    pub const SMSG_LOOT_ROLL: Opcode = Opcode { vanilla: 0x02A2, modern: 0x2617 }; // 674
    pub const CMSG_LOOT_MASTER_GIVE: Opcode = Opcode { vanilla: 0x02A3, modern: 0x320F }; // 675
    pub const SMSG_LOOT_MASTER_LIST: Opcode = Opcode { vanilla: 0x02A4, modern: 0x2618 }; // 676
    pub const SMSG_LOOT_ROLL_WON: Opcode = Opcode { vanilla: 0x029F, modern: 0x261B }; // 671
    pub const SMSG_LOOT_ALL_PASSED: Opcode = Opcode { vanilla: 0x029E, modern: 0x261A }; // 670

    // ============================================================================
    // Trade
    // ============================================================================

    pub const CMSG_INITIATE_TRADE: Opcode = Opcode { vanilla: 0x0116, modern: 0x3156 }; // 278
    pub const CMSG_BEGIN_TRADE: Opcode = Opcode { vanilla: 0x0117, modern: 0x3157 }; // 279
    pub const CMSG_BUSY_TRADE: Opcode = Opcode { vanilla: 0x0118, modern: 0x3158 }; // 280
    pub const CMSG_IGNORE_TRADE: Opcode = Opcode { vanilla: 0x0119, modern: 0x3159 }; // 281
    pub const CMSG_ACCEPT_TRADE: Opcode = Opcode { vanilla: 0x011A, modern: 0x315A }; // 282
    pub const CMSG_UNACCEPT_TRADE: Opcode = Opcode { vanilla: 0x011B, modern: 0x315B }; // 283
    pub const CMSG_CANCEL_TRADE: Opcode = Opcode { vanilla: 0x011C, modern: 0x315C }; // 284
    pub const CMSG_SET_TRADE_ITEM: Opcode = Opcode { vanilla: 0x011D, modern: 0x315D }; // 285
    pub const CMSG_CLEAR_TRADE_ITEM: Opcode = Opcode { vanilla: 0x011E, modern: 0x315E }; // 286
    pub const CMSG_SET_TRADE_GOLD: Opcode = Opcode { vanilla: 0x011F, modern: 0x315F }; // 287
    pub const SMSG_TRADE_STATUS: Opcode = Opcode { vanilla: 0x0120, modern: 0x2582 }; // 288
    pub const SMSG_TRADE_STATUS_EXTENDED: Opcode = Opcode { vanilla: 0x0121, ..Opcode::NONE }; // 289

    // ============================================================================
    // Quest
    // ============================================================================

    pub const CMSG_QUEST_QUERY: Opcode = Opcode { vanilla: 0x005C, modern: 0x326E }; // 92
    pub const SMSG_QUEST_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x005D, modern: 0x2A96 }; // 93
    pub const CMSG_QUESTGIVER_STATUS_QUERY: Opcode = Opcode { vanilla: 0x0182, modern: 0x349C }; // 386
    pub const SMSG_QUESTGIVER_STATUS: Opcode = Opcode { vanilla: 0x0183, modern: 0x2A9B }; // 387
    // Time sync. Modern-only: 1.12 has no equivalent, so both are `Opcode::NONE` on the vanilla
    // side. TrinityCore sends the request first in `SendInitialPacketsBeforeAddToMap`, then at 5 s
    // and every 10 s (`Player.cpp:24920-24928`, `WorldSession.cpp:1815-1826`); the client answers
    // with its own tick count so the server can measure clock drift.
    /// Sent when the client closes an interaction window (gossip, quest, vendor). Modern-only.
    pub const CMSG_CLOSE_INTERACTION: Opcode = Opcode { modern: 0x3493, ..Opcode::NONE };
    pub const SMSG_TIME_SYNC_REQUEST: Opcode = Opcode { modern: 0x2DD2, ..Opcode::NONE };
    pub const CMSG_TIME_SYNC_RESPONSE: Opcode = Opcode { modern: 0x3A3C, ..Opcode::NONE };
    pub const CMSG_TIME_SYNC_RESPONSE_FAILED: Opcode = Opcode { modern: 0x3A3D, ..Opcode::NONE };
    pub const CMSG_TIME_SYNC_RESPONSE_DROPPED: Opcode = Opcode { modern: 0x3A3E, ..Opcode::NONE };
    pub const CMSG_QUESTGIVER_STATUS_MULTIPLE_QUERY: Opcode = Opcode { modern: 0x349D, ..Opcode::NONE };
    pub const SMSG_QUESTGIVER_STATUS_MULTIPLE: Opcode = Opcode { modern: 0x2A91, ..Opcode::NONE };
    pub const CMSG_QUESTGIVER_HELLO: Opcode = Opcode { vanilla: 0x0184, modern: 0x3496 }; // 388
    pub const SMSG_QUESTGIVER_QUEST_LIST: Opcode = Opcode { vanilla: 0x0185, modern: 0x2A9A }; // 389
    pub const CMSG_QUESTGIVER_QUERY_QUEST: Opcode = Opcode { vanilla: 0x0186, modern: 0x3497 }; // 390
    pub const CMSG_QUESTGIVER_QUEST_AUTOLAUNCH: Opcode = Opcode { vanilla: 0x0187, ..Opcode::NONE }; // 391
    pub const SMSG_QUESTGIVER_QUEST_DETAILS: Opcode = Opcode { vanilla: 0x0188, modern: 0x2A92 }; // 392
    pub const CMSG_QUESTGIVER_ACCEPT_QUEST: Opcode = Opcode { vanilla: 0x0189, modern: 0x3498 }; // 393
    pub const CMSG_QUESTGIVER_COMPLETE_QUEST: Opcode = Opcode { vanilla: 0x018A, modern: 0x3499 }; // 394
    pub const SMSG_QUESTGIVER_REQUEST_ITEMS: Opcode = Opcode { vanilla: 0x018B, modern: 0x2A93 }; // 395
    pub const CMSG_QUESTGIVER_REQUEST_REWARD: Opcode = Opcode { vanilla: 0x018C, modern: 0x349B }; // 396
    pub const SMSG_QUESTGIVER_OFFER_REWARD: Opcode = Opcode { vanilla: 0x018D, modern: 0x2A94 }; // 397
    pub const CMSG_QUESTGIVER_CHOOSE_REWARD: Opcode = Opcode { vanilla: 0x018E, modern: 0x349A }; // 398
    pub const SMSG_QUESTGIVER_QUEST_INVALID: Opcode = Opcode { vanilla: 0x018F, modern: 0x2A85 }; // 399
    pub const CMSG_QUESTGIVER_CANCEL: Opcode = Opcode { vanilla: 0x0190, ..Opcode::NONE }; // 400
    pub const SMSG_QUESTGIVER_QUEST_COMPLETE: Opcode = Opcode { vanilla: 0x0191, modern: 0x2A83 }; // 401
    pub const SMSG_QUESTGIVER_QUEST_FAILED: Opcode = Opcode { vanilla: 0x0192, modern: 0x2A86 }; // 402
    pub const CMSG_QUESTLOG_SWAP_QUEST: Opcode = Opcode { vanilla: 0x0193, ..Opcode::NONE }; // 403
    pub const CMSG_QUESTLOG_REMOVE_QUEST: Opcode = Opcode { vanilla: 0x0194, modern: 0x352D }; // 404
    pub const SMSG_QUESTLOG_FULL: Opcode = Opcode { vanilla: 0x0195, modern: 0x2A87 }; // 405
    pub const SMSG_QUESTUPDATE_FAILED: Opcode = Opcode { vanilla: 0x0196, modern: 0x2A8A }; // 406
    pub const SMSG_QUESTUPDATE_FAILEDTIMER: Opcode = Opcode { vanilla: 0x0197, modern: 0x2A8B }; // 407
    pub const SMSG_QUESTUPDATE_COMPLETE: Opcode = Opcode { vanilla: 0x0198, modern: 0x2A89 }; // 408
    pub const SMSG_QUESTUPDATE_ADD_KILL: Opcode = Opcode { vanilla: 0x0199, modern: 0x2A8C }; // 409 (1.14 calls it SMSG_QUEST_UPDATE_ADD_CREDIT)
    pub const SMSG_QUESTUPDATE_ADD_ITEM: Opcode = Opcode { vanilla: 0x019A, ..Opcode::NONE }; // 410
    pub const CMSG_QUEST_CONFIRM_ACCEPT: Opcode = Opcode { vanilla: 0x019B, modern: 0x349E }; // 411
    pub const SMSG_QUEST_CONFIRM_ACCEPT: Opcode = Opcode { vanilla: 0x019C, modern: 0x2A8F }; // 412
    pub const CMSG_PUSHQUESTTOPARTY: Opcode = Opcode { vanilla: 0x019D, modern: 0x349F }; // 413
    pub const MSG_QUEST_PUSH_RESULT: Opcode = Opcode { vanilla: 0x0276, ..Opcode::NONE }; // 630

    // ============================================================================
    // Guild
    // ============================================================================

    pub const CMSG_GUILD_QUERY: Opcode = Opcode { vanilla: 0x0054, modern: 0x368B }; // 84
    pub const SMSG_GUILD_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x0055, modern: 0x29E5 }; // 85
    pub const CMSG_GUILD_CREATE: Opcode = Opcode { vanilla: 0x0081, ..Opcode::NONE }; // 129
    pub const CMSG_GUILD_INVITE: Opcode = Opcode { vanilla: 0x0082, modern: 0x3607 }; // 130
    pub const SMSG_GUILD_INVITE: Opcode = Opcode { vanilla: 0x0083, modern: 0x29CA }; // 131
    pub const CMSG_GUILD_ACCEPT: Opcode = Opcode { vanilla: 0x0084, modern: 0x35FD }; // 132
    pub const CMSG_GUILD_DECLINE: Opcode = Opcode { vanilla: 0x0085, modern: 0x3060 }; // 133
    pub const SMSG_GUILD_DECLINE: Opcode = Opcode { vanilla: 0x0086, modern: 0x29E8 }; // 134
    pub const CMSG_GUILD_INFO: Opcode = Opcode { vanilla: 0x0087, ..Opcode::NONE }; // 135
    pub const SMSG_GUILD_INFO: Opcode = Opcode { vanilla: 0x0088, ..Opcode::NONE }; // 136
    pub const CMSG_GUILD_ROSTER: Opcode = Opcode { vanilla: 0x0089, modern: 0x3073 }; // 137
    pub const SMSG_GUILD_ROSTER: Opcode = Opcode { vanilla: 0x008A, modern: 0x29BB }; // 138
    pub const CMSG_GUILD_PROMOTE: Opcode = Opcode { vanilla: 0x008B, modern: 0x305D }; // 139
    pub const CMSG_GUILD_DEMOTE: Opcode = Opcode { vanilla: 0x008C, modern: 0x305E }; // 140
    pub const CMSG_GUILD_LEAVE: Opcode = Opcode { vanilla: 0x008D, modern: 0x3062 }; // 141
    pub const CMSG_GUILD_REMOVE: Opcode = Opcode { vanilla: 0x008E, modern: 0x3063 }; // 142
    pub const CMSG_GUILD_DISBAND: Opcode = Opcode { vanilla: 0x008F, modern: 0x3068 }; // 143
    pub const CMSG_GUILD_LEADER: Opcode = Opcode { vanilla: 0x0090, modern: 0x36CE }; // 144
    pub const CMSG_GUILD_MOTD: Opcode = Opcode { vanilla: 0x0091, modern: 0x3074 }; // 145
    pub const SMSG_GUILD_EVENT: Opcode = Opcode { vanilla: 0x0092, ..Opcode::NONE }; // 146
    pub const SMSG_GUILD_COMMAND_RESULT: Opcode = Opcode { vanilla: 0x0093, modern: 0x29BA }; // 147
    pub const CMSG_GUILD_RANK: Opcode = Opcode { vanilla: 0x0231, modern: 0x3067 }; // 561
    pub const CMSG_GUILD_ADD_RANK: Opcode = Opcode { vanilla: 0x0232, modern: 0x3064 }; // 562
    pub const CMSG_GUILD_DEL_RANK: Opcode = Opcode { vanilla: 0x0233, modern: 0x3065 }; // 563
    pub const CMSG_GUILD_SET_PUBLIC_NOTE: Opcode = Opcode { vanilla: 0x0234, ..Opcode::NONE }; // 564
    pub const CMSG_GUILD_SET_OFFICER_NOTE: Opcode = Opcode { vanilla: 0x0235, ..Opcode::NONE }; // 565
    pub const CMSG_GUILD_INFO_TEXT: Opcode = Opcode { vanilla: 0x02FC, modern: 0x3075 }; // 764
    pub const MSG_SAVE_GUILD_EMBLEM: Opcode = Opcode { vanilla: 0x01F1, ..Opcode::NONE }; // 497

    // ============================================================================
    // Petition / Charter
    // ============================================================================

    pub const CMSG_PETITION_SHOWLIST: Opcode = Opcode { vanilla: 0x01BB, modern: 0x34C7 }; // 443
    pub const SMSG_PETITION_SHOWLIST: Opcode = Opcode { vanilla: 0x01BC, modern: 0x26B7 }; // 444
    pub const CMSG_PETITION_BUY: Opcode = Opcode { vanilla: 0x01BD, modern: 0x34C8 }; // 445
    pub const CMSG_PETITION_SHOW_SIGNATURES: Opcode = Opcode { vanilla: 0x01BE, modern: 0x34C9 }; // 446
    pub const SMSG_PETITION_SHOW_SIGNATURES: Opcode = Opcode { vanilla: 0x01BF, modern: 0x26B8 }; // 447
    pub const CMSG_PETITION_SIGN: Opcode = Opcode { vanilla: 0x01C0, modern: 0x3532 }; // 448
    pub const SMSG_PETITION_SIGN_RESULTS: Opcode = Opcode { vanilla: 0x01C1, modern: 0x2743 }; // 449
    pub const MSG_PETITION_DECLINE: Opcode = Opcode { vanilla: 0x01C2, ..Opcode::NONE }; // 450
    pub const SMSG_PETITION_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x01C3, ..Opcode::NONE }; // 451
    pub const CMSG_TURN_IN_PETITION: Opcode = Opcode { vanilla: 0x01C4, modern: 0x3534 }; // 452
    pub const SMSG_TURN_IN_PETITION_RESULTS: Opcode = Opcode { vanilla: 0x01C5, modern: 0x2745 }; // 453
    pub const CMSG_OFFER_PETITION: Opcode = Opcode { vanilla: 0x01C7, modern: 0x32F4 }; // 455
    pub const MSG_PETITION_RENAME: Opcode = Opcode { vanilla: 0x02C1, ..Opcode::NONE }; // 705

    // ============================================================================
    // Mail
    // ============================================================================

    pub const MSG_QUERY_NEXT_MAIL_TIME: Opcode = Opcode { vanilla: 0x0284, ..Opcode::NONE }; // 644
    pub const CMSG_SEND_MAIL: Opcode = Opcode { vanilla: 0x0238, modern: 0x35FB }; // 568
    pub const SMSG_SEND_MAIL_RESULT: Opcode = Opcode { vanilla: 0x0239, modern: 0x2634 }; // 569
    pub const CMSG_GET_MAIL_LIST: Opcode = Opcode { vanilla: 0x023A, modern: 0x3535 }; // 570
    pub const SMSG_MAIL_LIST_RESULT: Opcode = Opcode { vanilla: 0x023B, modern: 0x274D }; // 571
    pub const CMSG_MAIL_TAKE_MONEY: Opcode = Opcode { vanilla: 0x0245, modern: 0x3536 }; // 581
    pub const CMSG_MAIL_TAKE_ITEM: Opcode = Opcode { vanilla: 0x0246, modern: 0x3537 }; // 582
    pub const CMSG_MAIL_MARK_AS_READ: Opcode = Opcode { vanilla: 0x0247, modern: 0x3539 }; // 583
    pub const CMSG_MAIL_RETURN_TO_SENDER: Opcode = Opcode { vanilla: 0x0248, modern: 0x3658 }; // 584
    pub const CMSG_MAIL_DELETE: Opcode = Opcode { vanilla: 0x0249, modern: 0x3222 }; // 585
    pub const CMSG_MAIL_CREATE_TEXT_ITEM: Opcode = Opcode { vanilla: 0x024A, modern: 0x353A }; // 586
    pub const SMSG_RECEIVED_MAIL: Opcode = Opcode { vanilla: 0x0285, modern: 0x2635 }; // 645

    // ============================================================================
    // Auction House
    // ============================================================================

    pub const MSG_AUCTION_HELLO: Opcode = Opcode { vanilla: 0x0255, ..Opcode::NONE }; // 597
    pub const CMSG_AUCTION_SELL_ITEM: Opcode = Opcode { vanilla: 0x0256, modern: 0x34CB }; // 598
    pub const CMSG_AUCTION_REMOVE_ITEM: Opcode = Opcode { vanilla: 0x0257, modern: 0x34CC }; // 599
    pub const CMSG_AUCTION_LIST_ITEMS: Opcode = Opcode { vanilla: 0x0258, modern: 0x34CD }; // 600
    pub const CMSG_AUCTION_LIST_OWNER_ITEMS: Opcode = Opcode { vanilla: 0x0259, modern: 0x34CF }; // 601
    pub const CMSG_AUCTION_PLACE_BID: Opcode = Opcode { vanilla: 0x025A, modern: 0x34D1 }; // 602
    pub const SMSG_AUCTION_COMMAND_RESULT: Opcode = Opcode { vanilla: 0x025B, modern: 0x26E7 }; // 603
    pub const SMSG_AUCTION_LIST_RESULT: Opcode = Opcode { vanilla: 0x025C, modern: 0x2871 }; // 604
    pub const SMSG_AUCTION_OWNER_LIST_RESULT: Opcode = Opcode { vanilla: 0x025D, modern: 0x288B }; // 605
    pub const SMSG_AUCTION_BIDDER_NOTIFICATION: Opcode = Opcode { vanilla: 0x025E, ..Opcode::NONE }; // 606
    pub const SMSG_AUCTION_OWNER_NOTIFICATION: Opcode = Opcode { vanilla: 0x025F, ..Opcode::NONE }; // 607
    pub const CMSG_AUCTION_LIST_BIDDER_ITEMS: Opcode = Opcode { vanilla: 0x0264, modern: 0x34D0 }; // 612
    pub const SMSG_AUCTION_BIDDER_LIST_RESULT: Opcode = Opcode { vanilla: 0x0265, modern: 0x288C }; // 613
    pub const SMSG_AUCTION_REMOVED_NOTIFICATION: Opcode = Opcode { vanilla: 0x028D, ..Opcode::NONE }; // 653

    // ============================================================================
    // Battleground
    // ============================================================================

    pub const CMSG_BATTLEFIELD_STATUS: Opcode = Opcode { vanilla: 0x02D3, ..Opcode::NONE }; // 723
    pub const SMSG_BATTLEFIELD_STATUS: Opcode = Opcode { vanilla: 0x02D4, ..Opcode::NONE }; // 724
    pub const CMSG_BATTLEFIELD_LIST: Opcode = Opcode { vanilla: 0x023B, modern: 0x3182 }; // 571
    pub const SMSG_BATTLEFIELD_LIST: Opcode = Opcode { vanilla: 0x023C, modern: 0x2927 }; // 572
    pub const CMSG_BATTLEFIELD_JOIN: Opcode = Opcode { vanilla: 0x023E, ..Opcode::NONE }; // 574
    pub const SMSG_BATTLEFIELD_JOINED: Opcode = Opcode { vanilla: 0x02E1, ..Opcode::NONE }; // 737
    pub const SMSG_BATTLEFIELD_LEFT: Opcode = Opcode { vanilla: 0x02E2, ..Opcode::NONE }; // 738
    pub const CMSG_LEAVE_BATTLEFIELD: Opcode = Opcode { vanilla: 0x02E5, ..Opcode::NONE }; // 741
    pub const CMSG_BATTLEFIELD_PORT: Opcode = Opcode { vanilla: 0x02D5, modern: 0x3524 }; // 725
    pub const CMSG_BATTLEMASTER_HELLO: Opcode = Opcode { vanilla: 0x02D7, modern: 0x32A9 }; // 727
    pub const SMSG_BATTLEMASTER_JOINED: Opcode = Opcode { vanilla: 0x02E3, ..Opcode::NONE }; // 739
    pub const CMSG_BATTLEFIELD_QUEUE: Opcode = Opcode { vanilla: 0x023D, ..Opcode::NONE }; // 573
    pub const CMSG_BATTLEFIELD_UN_QUEUE: Opcode = Opcode { vanilla: 0x023F, ..Opcode::NONE }; // 575
    pub const CMSG_AREA_SPIRIT_HEALER_QUERY: Opcode = Opcode { vanilla: 0x02E2, modern: 0x34B0 }; // 738
    pub const CMSG_AREA_SPIRIT_HEALER_QUEUE: Opcode = Opcode { vanilla: 0x02E3, modern: 0x34B1 }; // 739
    pub const SMSG_AREA_SPIRIT_HEALER_TIME: Opcode = Opcode { vanilla: 0x02E4, modern: 0x2737 }; // 740

    // ============================================================================
    // Instance & Raid
    // ============================================================================

    pub const CMSG_REQUEST_RAID_INFO: Opcode = Opcode { vanilla: 0x02CD, modern: 0x36D0 }; // 717
    pub const SMSG_RAID_INSTANCE_INFO: Opcode = Opcode { vanilla: 0x02CC, modern: 0x262D }; // 716
    pub const CMSG_RESET_INSTANCES: Opcode = Opcode { vanilla: 0x031D, modern: 0x366A }; // 797
    pub const SMSG_INSTANCE_RESET: Opcode = Opcode { vanilla: 0x031E, modern: 0x267F }; // 798
    pub const SMSG_INSTANCE_RESET_FAILED: Opcode = Opcode { vanilla: 0x031F, modern: 0x2680 }; // 799

    // ============================================================================
    // Meeting Stone
    // ============================================================================

    pub const CMSG_MEETINGSTONE_INFO: Opcode = Opcode { vanilla: 0x0296, ..Opcode::NONE }; // 662
    pub const CMSG_MEETINGSTONE_JOIN: Opcode = Opcode { vanilla: 0x0292, ..Opcode::NONE }; // 658
    pub const CMSG_MEETINGSTONE_LEAVE: Opcode = Opcode { vanilla: 0x0293, ..Opcode::NONE }; // 659
    pub const CMSG_MEETINGSTONE_CHEAT: Opcode = Opcode { vanilla: 0x0294, ..Opcode::NONE }; // 660
    pub const SMSG_MEETINGSTONE_SETQUEUE: Opcode = Opcode { vanilla: 0x0295, ..Opcode::NONE }; // 661

    // ============================================================================
    // Duel
    // ============================================================================

    pub const CMSG_DUEL_ACCEPTED: Opcode = Opcode { vanilla: 0x016C, ..Opcode::NONE }; // 364
    pub const CMSG_DUEL_CANCELLED: Opcode = Opcode { vanilla: 0x016D, ..Opcode::NONE }; // 365
    pub const SMSG_DUEL_REQUESTED: Opcode = Opcode { vanilla: 0x0167, modern: 0x293E }; // 359
    pub const SMSG_DUEL_COUNTDOWN: Opcode = Opcode { vanilla: 0x02B7, modern: 0x2942 }; // 695
    pub const SMSG_DUEL_OUTOFBOUNDS: Opcode = Opcode { vanilla: 0x0168, modern: 0x2940 }; // 360
    pub const SMSG_DUEL_INBOUNDS: Opcode = Opcode { vanilla: 0x0169, modern: 0x2941 }; // 361
    pub const SMSG_DUEL_COMPLETE: Opcode = Opcode { vanilla: 0x016A, modern: 0x2943 }; // 362
    pub const SMSG_DUEL_WINNER: Opcode = Opcode { vanilla: 0x016B, modern: 0x2944 }; // 363

    // ============================================================================
    // Pet
    // ============================================================================

    pub const CMSG_PET_NAME_QUERY: Opcode = Opcode { vanilla: 0x0052, modern: 0x3270 }; // 82
    pub const SMSG_PET_NAME_QUERY_RESPONSE: Opcode = Opcode { vanilla: 0x0053, modern: 0x2919 }; // 83
    pub const CMSG_PET_SET_ACTION: Opcode = Opcode { vanilla: 0x0174, modern: 0x348A }; // 372
    pub const CMSG_PET_ACTION: Opcode = Opcode { vanilla: 0x0175, modern: 0x348B }; // 373
    pub const CMSG_PET_ABANDON: Opcode = Opcode { vanilla: 0x0176, modern: 0x348D }; // 374
    pub const CMSG_PET_RENAME: Opcode = Opcode { vanilla: 0x0177, modern: 0x3686 }; // 375
    pub const SMSG_PET_SPELLS: Opcode = Opcode { vanilla: 0x0179, modern: 0x2C25 }; // 377
    pub const SMSG_PET_MODE: Opcode = Opcode { vanilla: 0x017A, modern: 0x258C }; // 378
    pub const SMSG_PET_TAME_FAILURE: Opcode = Opcode { vanilla: 0x0173, modern: 0x26AC }; // 371
    pub const SMSG_PET_NAME_INVALID: Opcode = Opcode { vanilla: 0x0178, modern: 0x26BC }; // 376
    pub const CMSG_PET_CAST_SPELL: Opcode = Opcode { vanilla: 0x01F0, modern: 0x3293 }; // 496
    pub const SMSG_PET_CAST_FAILED: Opcode = Opcode { vanilla: 0x0138, modern: 0x2C58 }; // 312
    pub const CMSG_PET_CANCEL_AURA: Opcode = Opcode { vanilla: 0x026B, modern: 0x348E }; // 619
    pub const SMSG_PET_ACTION_FEEDBACK: Opcode = Opcode { vanilla: 0x02C6, modern: 0x2740 }; // 710
    pub const SMSG_PET_BROKEN: Opcode = Opcode { vanilla: 0x02B3, ..Opcode::NONE }; // 691
    pub const CMSG_PET_UNLEARN: Opcode = Opcode { vanilla: 0x02F0, ..Opcode::NONE }; // 752
    pub const SMSG_PET_UNLEARN_CONFIRM: Opcode = Opcode { vanilla: 0x02F1, ..Opcode::NONE }; // 753
    pub const CMSG_PET_SPELL_AUTOCAST: Opcode = Opcode { vanilla: 0x02F3, modern: 0x348F }; // 755
    pub const CMSG_PET_STOP_ATTACK: Opcode = Opcode { vanilla: 0x02EA, modern: 0x348C }; // 746
    pub const CMSG_REQUEST_PET_INFO: Opcode = Opcode { vanilla: 0x0279, modern: 0x3490 }; // 633

    // ============================================================================
    // Pet Stable
    // ============================================================================

    pub const MSG_LIST_STABLED_PETS: Opcode = Opcode { vanilla: 0x026E, ..Opcode::NONE }; // 623
    pub const CMSG_STABLE_PET: Opcode = Opcode { vanilla: 0x026F, modern: 0x3169 }; // 624
    pub const CMSG_UNSTABLE_PET: Opcode = Opcode { vanilla: 0x0270, modern: 0x316A }; // 625
    pub const CMSG_BUY_STABLE_SLOT: Opcode = Opcode { vanilla: 0x0272, modern: 0x316C }; // 626
    pub const SMSG_STABLE_RESULT: Opcode = Opcode { vanilla: 0x0273, modern: 0x2598 }; // 627
    pub const CMSG_STABLE_REVIVE_PET: Opcode = Opcode { vanilla: 0x0274, ..Opcode::NONE }; // 628
    pub const CMSG_STABLE_SWAP_PET: Opcode = Opcode { vanilla: 0x0275, modern: 0x316B }; // 629

    // ============================================================================
    // GM Ticket
    // ============================================================================

    pub const CMSG_GMTICKET_CREATE: Opcode = Opcode { vanilla: 0x0205, ..Opcode::NONE }; // 517
    pub const SMSG_GMTICKET_CREATE: Opcode = Opcode { vanilla: 0x0206, ..Opcode::NONE }; // 518
    pub const CMSG_GMTICKET_UPDATETEXT: Opcode = Opcode { vanilla: 0x0207, ..Opcode::NONE }; // 519
    pub const SMSG_GMTICKET_UPDATETEXT: Opcode = Opcode { vanilla: 0x0208, ..Opcode::NONE }; // 520
    pub const CMSG_GMTICKET_GETTICKET: Opcode = Opcode { vanilla: 0x0211, ..Opcode::NONE }; // 529
    pub const SMSG_GMTICKET_GETTICKET: Opcode = Opcode { vanilla: 0x0212, ..Opcode::NONE }; // 530
    pub const CMSG_GMTICKET_DELETETICKET: Opcode = Opcode { vanilla: 0x0217, ..Opcode::NONE }; // 535
    pub const SMSG_GMTICKET_DELETETICKET: Opcode = Opcode { vanilla: 0x0218, ..Opcode::NONE }; // 536
    pub const CMSG_GMTICKET_SYSTEMSTATUS: Opcode = Opcode { vanilla: 0x021A, modern: 0x368E }; // 538
    pub const SMSG_GMTICKET_SYSTEMSTATUS: Opcode = Opcode { vanilla: 0x021B, modern: 0x269B }; // 539
    pub const CMSG_GMSURVEY_SUBMIT: Opcode = Opcode { vanilla: 0x032A, ..Opcode::NONE }; // 810

    // ============================================================================
    // PvP
    // ============================================================================

    pub const CMSG_TOGGLE_PVP: Opcode = Opcode { vanilla: 0x0253, modern: 0x32A3 }; // 595

    // ============================================================================
    // Summon
    // ============================================================================

    pub const SMSG_SUMMON_REQUEST: Opcode = Opcode { vanilla: 0x02AB, modern: 0x2718 }; // 683
    pub const CMSG_SUMMON_RESPONSE: Opcode = Opcode { vanilla: 0x02AC, modern: 0x366C }; // 684

    // ============================================================================
    // Far Sight
    // ============================================================================

    pub const CMSG_FAR_SIGHT: Opcode = Opcode { vanilla: 0x027A, modern: 0x34E8 }; // 634

    // ============================================================================
    // Appearance
    // ============================================================================

    pub const CMSG_TOGGLE_HELM: Opcode = Opcode { vanilla: 0x02B9, modern: 0x3563 }; // 697
    pub const CMSG_TOGGLE_CLOAK: Opcode = Opcode { vanilla: 0x02BA, modern: 0x3564 }; // 698

    // ============================================================================
    // Player Misc
    // ============================================================================

    pub const CMSG_SAVE_PLAYER: Opcode = Opcode { vanilla: 0x0153, ..Opcode::NONE }; // 339
    pub const CMSG_SETSHEATHED: Opcode = Opcode { vanilla: 0x01E0, modern: 0x3489 }; // 480
    pub const CMSG_GHOST: Opcode = Opcode { vanilla: 0x01E5, ..Opcode::NONE }; // 485
    pub const CMSG_PLAYED_TIME: Opcode = Opcode { vanilla: 0x01CC, modern: 0x3275 }; // 460
    pub const SMSG_PLAYED_TIME: Opcode = Opcode { vanilla: 0x01CD, modern: 0x26CD }; // 461
    pub const CMSG_BUG: Opcode = Opcode { vanilla: 0x01CA, ..Opcode::NONE }; // 458

    // ============================================================================
    // Warden (Anticheat)
    // ============================================================================

    pub const CMSG_WARDEN_DATA: Opcode = Opcode { vanilla: 0x02E7, ..Opcode::NONE }; // 743
    pub const SMSG_WARDEN_DATA: Opcode = Opcode { vanilla: 0x02E6, ..Opcode::NONE }; // 742

    // ============================================================================
    // Weather
    // ============================================================================

    pub const SMSG_WEATHER: Opcode = Opcode { vanilla: 0x02F4, modern: 0x269F }; // 756

    // ============================================================================
    // Modern-only
    //
    // No 1.12 counterpart, so `vanilla` stays 0. Added by hand as handlers need them rather than
    // bulk-imported: the modern table has ~1730 entries, and importing the lot would bury the
    // opcodes we actually serve under dead ones.
    // ============================================================================

    // Handshake. Also mirrored as bare u16s in world's `modern/opcodes.rs`, which predates this
    // table; that module should fold into these once the modern socket uses `Opcode` throughout.
    pub const SMSG_ENTER_ENCRYPTED_MODE: Opcode = Opcode { modern: 0x3049, ..Opcode::NONE };
    pub const CMSG_ENTER_ENCRYPTED_MODE_ACK: Opcode = Opcode { modern: 0x3767, ..Opcode::NONE };
    pub const CMSG_AUTH_CONTINUED_SESSION: Opcode = Opcode { modern: 0x3766, ..Opcode::NONE };
    pub const SMSG_CONNECT_TO: Opcode = Opcode { modern: 0x304D, ..Opcode::NONE };
    pub const SMSG_RESUME_COMMS: Opcode = Opcode { modern: 0x304B, ..Opcode::NONE };
    pub const CMSG_CONNECT_TO_FAILED: Opcode = Opcode { modern: 0x35D4, ..Opcode::NONE };

    // Movement. 1.14 split vanilla's bidirectional MSG_MOVE_* pairs: the client still sends the
    // shared number, but the server answers observers on a dedicated SMSG. There is no vanilla
    // equivalent -- vanilla simply echoes the client's own opcode back to nearby players.
    pub const SMSG_MOVE_UPDATE: Opcode = Opcode { modern: 0x2DE0, ..Opcode::NONE };

    // Enter-world packets with no 1.12 counterpart. The client waits on these before it will
    // finish loading, which is why a login that looks complete server-side can still hang or die.
    pub const SMSG_WORLD_SERVER_INFO: Opcode = Opcode { modern: 0x25AC, ..Opcode::NONE };
    pub const SMSG_SET_ALL_TASK_PROGRESS: Opcode = Opcode { modern: 0x2789, ..Opcode::NONE };
    pub const SMSG_INITIAL_SETUP: Opcode = Opcode { modern: 0x2580, ..Opcode::NONE };
    pub const SMSG_LOAD_CUF_PROFILES: Opcode = Opcode { modern: 0x25B9, ..Opcode::NONE };

    // Glue screen. The client sends all of these unprompted at character select.
    pub const CMSG_SERVER_TIME_OFFSET_REQUEST: Opcode = Opcode { modern: 0x369B, ..Opcode::NONE };
    pub const SMSG_SERVER_TIME_OFFSET: Opcode = Opcode { modern: 0x270B, ..Opcode::NONE };
    pub const CMSG_UPDATE_VAS_PURCHASE_STATES: Opcode = Opcode { modern: 0x36F9, ..Opcode::NONE };
    pub const CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS: Opcode = Opcode { modern: 0x36E5, ..Opcode::NONE };
    pub const CMSG_BATTLE_PAY_GET_PRODUCT_LIST: Opcode = Opcode { modern: 0x36C2, ..Opcode::NONE };
    pub const CMSG_BATTLE_PAY_GET_PURCHASE_LIST: Opcode = Opcode { modern: 0x36C3, ..Opcode::NONE };

    // Transport diagnostic sent by the modern client while it disconnects.
    pub const CMSG_LOG_DISCONNECT: Opcode = Opcode { modern: 0x3769, ..Opcode::NONE };
    pub const CMSG_QUEUED_MESSAGES_END: Opcode = Opcode { modern: 0x376C, ..Opcode::NONE };
    pub const CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE: Opcode = Opcode { modern: 0x3A45, ..Opcode::NONE };

    // Client bootstrap and optional service requests. These are registered even where oxcore has
    // no backing system, so it can return the protocol-defined empty response rather than leave a
    // modern client waiting on an unresolved wire opcode.
    pub const CMSG_REQUEST_CATEGORY_COOLDOWNS: Opcode = Opcode { modern: 0x3181, ..Opcode::NONE };
    pub const SMSG_SEND_SPELL_HISTORY: Opcode = Opcode { modern: 0x2C2B, ..Opcode::NONE };
    pub const CMSG_REQUEST_FORCED_REACTIONS: Opcode = Opcode { modern: 0x3207, ..Opcode::NONE };
    pub const CMSG_QUERY_NEXT_MAIL_TIME: Opcode = Opcode { modern: 0x3538, ..Opcode::NONE };
    pub const SMSG_MAIL_QUERY_NEXT_TIME_RESULT: Opcode = Opcode { modern: 0x274E, ..Opcode::NONE };
    pub const CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS: Opcode = Opcode { modern: 0x32AC, ..Opcode::NONE };
    pub const SMSG_CONQUEST_FORMULA_CONSTANTS: Opcode = Opcode { modern: 0x2780, ..Opcode::NONE };
    pub const CMSG_REQUEST_LFG_LIST_BLACKLIST: Opcode = Opcode { modern: 0x329C, ..Opcode::NONE };
    pub const SMSG_LFG_LIST_UPDATE_BLACKLIST: Opcode = Opcode { modern: 0x2A2A, ..Opcode::NONE };
    pub const CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY: Opcode = Opcode { modern: 0x3083, ..Opcode::NONE };
    pub const SMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY: Opcode = Opcode { modern: 0x29E0, ..Opcode::NONE };
    pub const CMSG_CALENDAR_GET_NUM_PENDING: Opcode = Opcode { modern: 0x367C, ..Opcode::NONE };
    pub const SMSG_CALENDAR_SEND_NUM_PENDING: Opcode = Opcode { modern: 0x2695, ..Opcode::NONE };
    pub const CMSG_GET_ACCOUNT_CHARACTER_LIST: Opcode = Opcode { modern: 0x36BD, ..Opcode::NONE };
    pub const SMSG_GET_ACCOUNT_CHARACTER_LIST_RESULT: Opcode = Opcode { modern: 0x275C, ..Opcode::NONE };
    pub const CMSG_LOADING_SCREEN_NOTIFY: Opcode = Opcode { modern: 0x35F9, ..Opcode::NONE };
    pub const CMSG_CHAT_REGISTER_ADDON_PREFIXES: Opcode = Opcode { modern: 0x37CD, ..Opcode::NONE };
    pub const CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES: Opcode = Opcode { modern: 0x37CE, ..Opcode::NONE };
    pub const CMSG_VIOLENCE_LEVEL: Opcode = Opcode { modern: 0x3188, ..Opcode::NONE };
    pub const CMSG_QUERY_COUNTDOWN_TIMER: Opcode = Opcode { modern: 0x31AB, ..Opcode::NONE };
    pub const CMSG_REQUEST_CEMETERY_LIST: Opcode = Opcode { modern: 0x3179, ..Opcode::NONE };
    pub const CMSG_REQUEST_BATTLEFIELD_STATUS: Opcode = Opcode { modern: 0x35DD, ..Opcode::NONE };
    pub const CMSG_LFG_LIST_GET_STATUS: Opcode = Opcode { modern: 0x360C, ..Opcode::NONE };
    pub const CMSG_BATTLE_PET_REQUEST_JOURNAL: Opcode = Opcode { modern: 0x3625, ..Opcode::NONE };
    pub const CMSG_ARENA_TEAM_ACCEPT: Opcode = Opcode { modern: 0x36B7, ..Opcode::NONE };
    pub const CMSG_GUILD_SET_ACHIEVEMENT_TRACKING: Opcode = Opcode { modern: 0x306F, ..Opcode::NONE };
    pub const CMSG_GM_TICKET_GET_CASE_STATUS: Opcode = Opcode { modern: 0x368F, ..Opcode::NONE };
}

/// Prints the constant's name — `CMSG_PING` rather than `Opcode(476)`. Every dispatcher logs
/// opcodes, and a bare number is unreadable across two protocols that number them differently.
impl std::fmt::Debug for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name() {
            Some(name) => f.write_str(name),
            None => write!(
                f,
                "Opcode(vanilla=0x{:04X}, modern=0x{:04X})",
                self.vanilla, self.modern
            ),
        }
    }
}

/// Every opcode constant paired with its name, in declaration order.
///
/// Backs both the wire lookups and `Debug`. A test asserts this stays the same length as the
/// constant list above, so the two cannot drift apart.
pub const ALL: &[(Opcode, &str)] = &[
    (Opcode::CMSG_NULL_ACTION, "CMSG_NULL_ACTION"),
    (Opcode::CMSG_PING, "CMSG_PING"),
    (Opcode::CMSG_AUTH_SESSION, "CMSG_AUTH_SESSION"),
    (Opcode::SMSG_AUTH_CHALLENGE, "SMSG_AUTH_CHALLENGE"),
    (Opcode::SMSG_AUTH_RESPONSE, "SMSG_AUTH_RESPONSE"),
    (Opcode::SMSG_PONG, "SMSG_PONG"),
    (Opcode::CMSG_CHAR_CREATE, "CMSG_CHAR_CREATE"),
    (Opcode::CMSG_CHAR_ENUM, "CMSG_CHAR_ENUM"),
    (Opcode::CMSG_CHAR_DELETE, "CMSG_CHAR_DELETE"),
    (Opcode::CMSG_PLAYER_LOGIN, "CMSG_PLAYER_LOGIN"),
    (Opcode::CMSG_CHAR_RENAME, "CMSG_CHAR_RENAME"),
    (Opcode::SMSG_CHAR_CREATE, "SMSG_CHAR_CREATE"),
    (Opcode::SMSG_CHAR_ENUM, "SMSG_CHAR_ENUM"),
    (Opcode::SMSG_CHAR_DELETE, "SMSG_CHAR_DELETE"),
    (Opcode::SMSG_CHAR_RENAME, "SMSG_CHAR_RENAME"),
    (
        Opcode::SMSG_CHARACTER_LOGIN_FAILED,
        "SMSG_CHARACTER_LOGIN_FAILED",
    ),
    (Opcode::CMSG_LOGOUT_REQUEST, "CMSG_LOGOUT_REQUEST"),
    (Opcode::CMSG_LOGOUT_CANCEL, "CMSG_LOGOUT_CANCEL"),
    (Opcode::SMSG_LOGOUT_RESPONSE, "SMSG_LOGOUT_RESPONSE"),
    (Opcode::SMSG_LOGOUT_COMPLETE, "SMSG_LOGOUT_COMPLETE"),
    (Opcode::SMSG_LOGOUT_CANCEL_ACK, "SMSG_LOGOUT_CANCEL_ACK"),
    (Opcode::SMSG_NEW_WORLD, "SMSG_NEW_WORLD"),
    (Opcode::SMSG_TRANSFER_PENDING, "SMSG_TRANSFER_PENDING"),
    (Opcode::SMSG_LOGIN_SETTIMESPEED, "SMSG_LOGIN_SETTIMESPEED"),
    (Opcode::SMSG_LOGIN_VERIFY_WORLD, "SMSG_LOGIN_VERIFY_WORLD"),
    (Opcode::CMSG_QUERY_TIME, "CMSG_QUERY_TIME"),
    (Opcode::SMSG_QUERY_TIME_RESPONSE, "SMSG_QUERY_TIME_RESPONSE"),
    (Opcode::SMSG_TUTORIAL_FLAGS, "SMSG_TUTORIAL_FLAGS"),
    (Opcode::CMSG_TUTORIAL_FLAG, "CMSG_TUTORIAL_FLAG"),
    (Opcode::CMSG_TUTORIAL_CLEAR, "CMSG_TUTORIAL_CLEAR"),
    (Opcode::CMSG_TUTORIAL_RESET, "CMSG_TUTORIAL_RESET"),
    (Opcode::CMSG_UPDATE_ACCOUNT_DATA, "CMSG_UPDATE_ACCOUNT_DATA"),
    (Opcode::SMSG_UPDATE_ACCOUNT_DATA, "SMSG_UPDATE_ACCOUNT_DATA"),
    (
        Opcode::CMSG_REQUEST_ACCOUNT_DATA,
        "CMSG_REQUEST_ACCOUNT_DATA",
    ),
    (
        Opcode::SMSG_UPDATE_ACCOUNT_DATA_COMPLETE,
        "SMSG_UPDATE_ACCOUNT_DATA_COMPLETE",
    ),
    (Opcode::SMSG_ACCOUNT_DATA_MD5, "SMSG_ACCOUNT_DATA_MD5"),
    (Opcode::SMSG_ACCOUNT_DATA_TIMES, "SMSG_ACCOUNT_DATA_TIMES"),
    (Opcode::CMSG_NAME_QUERY, "CMSG_NAME_QUERY"),
    (Opcode::SMSG_NAME_QUERY_RESPONSE, "SMSG_NAME_QUERY_RESPONSE"),
    (Opcode::CMSG_CREATURE_QUERY, "CMSG_CREATURE_QUERY"),
    (
        Opcode::SMSG_CREATURE_QUERY_RESPONSE,
        "SMSG_CREATURE_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_ITEM_QUERY_SINGLE, "CMSG_ITEM_QUERY_SINGLE"),
    (Opcode::CMSG_ITEM_QUERY_MULTIPLE, "CMSG_ITEM_QUERY_MULTIPLE"),
    (
        Opcode::SMSG_ITEM_QUERY_SINGLE_RESPONSE,
        "SMSG_ITEM_QUERY_SINGLE_RESPONSE",
    ),
    (
        Opcode::SMSG_ITEM_QUERY_MULTIPLE_RESPONSE,
        "SMSG_ITEM_QUERY_MULTIPLE_RESPONSE",
    ),
    (Opcode::CMSG_GAMEOBJECT_QUERY, "CMSG_GAMEOBJECT_QUERY"),
    (
        Opcode::SMSG_GAMEOBJECT_QUERY_RESPONSE,
        "SMSG_GAMEOBJECT_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_PAGE_TEXT_QUERY, "CMSG_PAGE_TEXT_QUERY"),
    (
        Opcode::SMSG_PAGE_TEXT_QUERY_RESPONSE,
        "SMSG_PAGE_TEXT_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_ITEM_TEXT_QUERY, "CMSG_ITEM_TEXT_QUERY"),
    (
        Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE,
        "SMSG_ITEM_TEXT_QUERY_RESPONSE",
    ),
    (Opcode::MSG_CORPSE_QUERY, "MSG_CORPSE_QUERY"),
    (Opcode::CMSG_ITEM_NAME_QUERY, "CMSG_ITEM_NAME_QUERY"),
    (
        Opcode::SMSG_ITEM_NAME_QUERY_RESPONSE,
        "SMSG_ITEM_NAME_QUERY_RESPONSE",
    ),
    (Opcode::SMSG_UPDATE_OBJECT, "SMSG_UPDATE_OBJECT"),
    (
        Opcode::SMSG_COMPRESSED_UPDATE_OBJECT,
        "SMSG_COMPRESSED_UPDATE_OBJECT",
    ),
    (Opcode::SMSG_COMPRESSED_MOVES, "SMSG_COMPRESSED_MOVES"),
    (Opcode::SMSG_DESTROY_OBJECT, "SMSG_DESTROY_OBJECT"),
    (Opcode::MSG_MOVE_HEARTBEAT, "MSG_MOVE_HEARTBEAT"),
    (Opcode::MSG_MOVE_START_FORWARD, "MSG_MOVE_START_FORWARD"),
    (Opcode::MSG_MOVE_START_BACKWARD, "MSG_MOVE_START_BACKWARD"),
    (Opcode::MSG_MOVE_STOP, "MSG_MOVE_STOP"),
    (
        Opcode::MSG_MOVE_START_STRAFE_LEFT,
        "MSG_MOVE_START_STRAFE_LEFT",
    ),
    (
        Opcode::MSG_MOVE_START_STRAFE_RIGHT,
        "MSG_MOVE_START_STRAFE_RIGHT",
    ),
    (Opcode::MSG_MOVE_STOP_STRAFE, "MSG_MOVE_STOP_STRAFE"),
    (Opcode::MSG_MOVE_JUMP, "MSG_MOVE_JUMP"),
    (Opcode::MSG_MOVE_START_TURN_LEFT, "MSG_MOVE_START_TURN_LEFT"),
    (
        Opcode::MSG_MOVE_START_TURN_RIGHT,
        "MSG_MOVE_START_TURN_RIGHT",
    ),
    (Opcode::MSG_MOVE_STOP_TURN, "MSG_MOVE_STOP_TURN"),
    (Opcode::MSG_MOVE_SET_FACING, "MSG_MOVE_SET_FACING"),
    (Opcode::MSG_MOVE_SET_PITCH, "MSG_MOVE_SET_PITCH"),
    (Opcode::MSG_MOVE_WORLDPORT_ACK, "MSG_MOVE_WORLDPORT_ACK"),
    (Opcode::MSG_MOVE_FALL_LAND, "MSG_MOVE_FALL_LAND"),
    (Opcode::CMSG_SET_ACTIVE_MOVER, "CMSG_SET_ACTIVE_MOVER"),
    (Opcode::CMSG_MOVE_SPLINE_DONE, "CMSG_MOVE_SPLINE_DONE"),
    (Opcode::CMSG_MOVE_FALL_RESET, "CMSG_MOVE_FALL_RESET"),
    (Opcode::CMSG_MOVE_TIME_SKIPPED, "CMSG_MOVE_TIME_SKIPPED"),
    (
        Opcode::CMSG_MOVE_FEATHER_FALL_ACK,
        "CMSG_MOVE_FEATHER_FALL_ACK",
    ),
    (Opcode::CMSG_MOVE_WATER_WALK_ACK, "CMSG_MOVE_WATER_WALK_ACK"),
    (
        Opcode::CMSG_MOVE_NOT_ACTIVE_MOVER,
        "CMSG_MOVE_NOT_ACTIVE_MOVER",
    ),
    (Opcode::MSG_MOVE_TELEPORT_ACK, "MSG_MOVE_TELEPORT_ACK"),
    (Opcode::MSG_MOVE_TELEPORT, "MSG_MOVE_TELEPORT"),
    (Opcode::MSG_MOVE_KNOCK_BACK, "MSG_MOVE_KNOCK_BACK"),
    (Opcode::MSG_MOVE_TIME_SKIPPED, "MSG_MOVE_TIME_SKIPPED"),
    (Opcode::CMSG_MOUNTSPECIAL_ANIM, "CMSG_MOUNTSPECIAL_ANIM"),
    (Opcode::SMSG_MOUNTSPECIAL_ANIM, "SMSG_MOUNTSPECIAL_ANIM"),
    (
        Opcode::SMSG_FORCE_WALK_SPEED_CHANGE,
        "SMSG_FORCE_WALK_SPEED_CHANGE",
    ),
    (
        Opcode::SMSG_FORCE_RUN_SPEED_CHANGE,
        "SMSG_FORCE_RUN_SPEED_CHANGE",
    ),
    (
        Opcode::SMSG_FORCE_RUN_BACK_SPEED_CHANGE,
        "SMSG_FORCE_RUN_BACK_SPEED_CHANGE",
    ),
    (
        Opcode::SMSG_FORCE_SWIM_SPEED_CHANGE,
        "SMSG_FORCE_SWIM_SPEED_CHANGE",
    ),
    (
        Opcode::SMSG_FORCE_SWIM_BACK_SPEED_CHANGE,
        "SMSG_FORCE_SWIM_BACK_SPEED_CHANGE",
    ),
    (
        Opcode::SMSG_FORCE_TURN_RATE_CHANGE,
        "SMSG_FORCE_TURN_RATE_CHANGE",
    ),
    (
        Opcode::CMSG_FORCE_WALK_SPEED_CHANGE_ACK,
        "CMSG_FORCE_WALK_SPEED_CHANGE_ACK",
    ),
    (
        Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK,
        "CMSG_FORCE_RUN_SPEED_CHANGE_ACK",
    ),
    (
        Opcode::CMSG_FORCE_RUN_BACK_SPEED_CHANGE_ACK,
        "CMSG_FORCE_RUN_BACK_SPEED_CHANGE_ACK",
    ),
    (
        Opcode::CMSG_FORCE_SWIM_SPEED_CHANGE_ACK,
        "CMSG_FORCE_SWIM_SPEED_CHANGE_ACK",
    ),
    (
        Opcode::CMSG_FORCE_SWIM_BACK_SPEED_CHANGE_ACK,
        "CMSG_FORCE_SWIM_BACK_SPEED_CHANGE_ACK",
    ),
    (
        Opcode::CMSG_FORCE_TURN_RATE_CHANGE_ACK,
        "CMSG_FORCE_TURN_RATE_CHANGE_ACK",
    ),
    (
        Opcode::SMSG_SPLINE_SET_WALK_SPEED,
        "SMSG_SPLINE_SET_WALK_SPEED",
    ),
    (
        Opcode::SMSG_SPLINE_SET_RUN_SPEED,
        "SMSG_SPLINE_SET_RUN_SPEED",
    ),
    (
        Opcode::SMSG_SPLINE_SET_RUN_BACK_SPEED,
        "SMSG_SPLINE_SET_RUN_BACK_SPEED",
    ),
    (
        Opcode::SMSG_SPLINE_SET_SWIM_SPEED,
        "SMSG_SPLINE_SET_SWIM_SPEED",
    ),
    (
        Opcode::SMSG_SPLINE_SET_SWIM_BACK_SPEED,
        "SMSG_SPLINE_SET_SWIM_BACK_SPEED",
    ),
    (
        Opcode::SMSG_SPLINE_SET_TURN_RATE,
        "SMSG_SPLINE_SET_TURN_RATE",
    ),
    (Opcode::MSG_MOVE_SET_WALK_SPEED, "MSG_MOVE_SET_WALK_SPEED"),
    (Opcode::MSG_MOVE_SET_RUN_SPEED, "MSG_MOVE_SET_RUN_SPEED"),
    (
        Opcode::MSG_MOVE_SET_RUN_BACK_SPEED,
        "MSG_MOVE_SET_RUN_BACK_SPEED",
    ),
    (Opcode::MSG_MOVE_SET_SWIM_SPEED, "MSG_MOVE_SET_SWIM_SPEED"),
    (
        Opcode::MSG_MOVE_SET_SWIM_BACK_SPEED,
        "MSG_MOVE_SET_SWIM_BACK_SPEED",
    ),
    (Opcode::MSG_MOVE_SET_TURN_RATE, "MSG_MOVE_SET_TURN_RATE"),
    (Opcode::SMSG_FORCE_MOVE_ROOT, "SMSG_FORCE_MOVE_ROOT"),
    (Opcode::CMSG_FORCE_MOVE_ROOT_ACK, "CMSG_FORCE_MOVE_ROOT_ACK"),
    (Opcode::SMSG_FORCE_MOVE_UNROOT, "SMSG_FORCE_MOVE_UNROOT"),
    (Opcode::SMSG_MOVE_WATER_WALK, "SMSG_MOVE_WATER_WALK"),
    (Opcode::SMSG_MOVE_LAND_WALK, "SMSG_MOVE_LAND_WALK"),
    (Opcode::SMSG_MOVE_SET_HOVER, "SMSG_MOVE_SET_HOVER"),
    (Opcode::SMSG_MOVE_UNSET_HOVER, "SMSG_MOVE_UNSET_HOVER"),
    (Opcode::SMSG_MOVE_FEATHER_FALL, "SMSG_MOVE_FEATHER_FALL"),
    (Opcode::SMSG_MOVE_NORMAL_FALL, "SMSG_MOVE_NORMAL_FALL"),
    (Opcode::SMSG_MOVE_KNOCK_BACK, "SMSG_MOVE_KNOCK_BACK"),
    (Opcode::CMSG_MOVE_KNOCK_BACK_ACK, "CMSG_MOVE_KNOCK_BACK_ACK"),
    (Opcode::SMSG_SPLINE_MOVE_ROOT, "SMSG_SPLINE_MOVE_ROOT"),
    (Opcode::SMSG_SPLINE_MOVE_UNROOT, "SMSG_SPLINE_MOVE_UNROOT"),
    (
        Opcode::SMSG_SPLINE_MOVE_WATER_WALK,
        "SMSG_SPLINE_MOVE_WATER_WALK",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_LAND_WALK,
        "SMSG_SPLINE_MOVE_LAND_WALK",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_SET_HOVER,
        "SMSG_SPLINE_MOVE_SET_HOVER",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_UNSET_HOVER,
        "SMSG_SPLINE_MOVE_UNSET_HOVER",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_FEATHER_FALL,
        "SMSG_SPLINE_MOVE_FEATHER_FALL",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_NORMAL_FALL,
        "SMSG_SPLINE_MOVE_NORMAL_FALL",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_SET_RUN_MODE,
        "SMSG_SPLINE_MOVE_SET_RUN_MODE",
    ),
    (
        Opcode::SMSG_SPLINE_MOVE_SET_WALK_MODE,
        "SMSG_SPLINE_MOVE_SET_WALK_MODE",
    ),
    (Opcode::MSG_MOVE_ROOT, "MSG_MOVE_ROOT"),
    (Opcode::MSG_MOVE_UNROOT, "MSG_MOVE_UNROOT"),
    (Opcode::MSG_MOVE_WATER_WALK, "MSG_MOVE_WATER_WALK"),
    (Opcode::MSG_MOVE_HOVER, "MSG_MOVE_HOVER"),
    (Opcode::MSG_MOVE_FEATHER_FALL, "MSG_MOVE_FEATHER_FALL"),
    (Opcode::SMSG_MONSTER_MOVE, "SMSG_MONSTER_MOVE"),
    (
        Opcode::SMSG_MONSTER_MOVE_TRANSPORT,
        "SMSG_MONSTER_MOVE_TRANSPORT",
    ),
    (Opcode::CMSG_ATTACKSWING, "CMSG_ATTACKSWING"),
    (Opcode::CMSG_ATTACKSTOP, "CMSG_ATTACKSTOP"),
    (Opcode::SMSG_ATTACKSTART, "SMSG_ATTACKSTART"),
    (Opcode::SMSG_ATTACKSTOP, "SMSG_ATTACKSTOP"),
    (
        Opcode::SMSG_ATTACKSWING_NOTINRANGE,
        "SMSG_ATTACKSWING_NOTINRANGE",
    ),
    (
        Opcode::SMSG_ATTACKSWING_BADFACING,
        "SMSG_ATTACKSWING_BADFACING",
    ),
    (
        Opcode::SMSG_ATTACKSWING_NOTSTANDING,
        "SMSG_ATTACKSWING_NOTSTANDING",
    ),
    (
        Opcode::SMSG_ATTACKSWING_DEADTARGET,
        "SMSG_ATTACKSWING_DEADTARGET",
    ),
    (
        Opcode::SMSG_ATTACKSWING_CANT_ATTACK,
        "SMSG_ATTACKSWING_CANT_ATTACK",
    ),
    (Opcode::SMSG_ATTACKERSTATEUPDATE, "SMSG_ATTACKERSTATEUPDATE"),
    (Opcode::CMSG_SET_SELECTION, "CMSG_SET_SELECTION"),
    (Opcode::CMSG_STANDSTATECHANGE, "CMSG_STANDSTATECHANGE"),
    (Opcode::SMSG_STANDSTATE_UPDATE, "SMSG_STANDSTATE_UPDATE"),
    (Opcode::CMSG_CAST_SPELL, "CMSG_CAST_SPELL"),
    (Opcode::CMSG_CANCEL_CAST, "CMSG_CANCEL_CAST"),
    (Opcode::CMSG_CANCEL_AURA, "CMSG_CANCEL_AURA"),
    (Opcode::CMSG_CANCEL_GROWTH_AURA, "CMSG_CANCEL_GROWTH_AURA"),
    (
        Opcode::CMSG_CANCEL_AUTO_REPEAT_SPELL,
        "CMSG_CANCEL_AUTO_REPEAT_SPELL",
    ),
    (Opcode::SMSG_CANCEL_AUTO_REPEAT, "SMSG_CANCEL_AUTO_REPEAT"),
    (Opcode::CMSG_CANCEL_CHANNELING, "CMSG_CANCEL_CHANNELING"),
    (Opcode::CMSG_CANCEL_CHANNELLING, "CMSG_CANCEL_CHANNELLING"),
    (Opcode::CMSG_USE_ITEM, "CMSG_USE_ITEM"),
    (Opcode::CMSG_NEW_SPELL_SLOT, "CMSG_NEW_SPELL_SLOT"),
    (Opcode::SMSG_SPELL_START, "SMSG_SPELL_START"),
    (Opcode::SMSG_SPELL_GO, "SMSG_SPELL_GO"),
    (Opcode::SMSG_CAST_RESULT, "SMSG_CAST_RESULT"),
    (Opcode::SMSG_SPELL_COOLDOWN, "SMSG_SPELL_COOLDOWN"),
    (Opcode::MSG_CHANNEL_START, "MSG_CHANNEL_START"),
    (Opcode::MSG_CHANNEL_UPDATE, "MSG_CHANNEL_UPDATE"),
    (Opcode::SMSG_SPELL_INTERRUPTED, "SMSG_SPELL_INTERRUPTED"),
    (Opcode::SMSG_SPELL_DELAYED, "SMSG_SPELL_DELAYED"),
    (Opcode::SMSG_SPELL_FAILED_OTHER, "SMSG_SPELL_FAILED_OTHER"),
    (
        Opcode::SMSG_SPELL_UPDATE_CHAIN_TARGETS,
        "SMSG_SPELL_UPDATE_CHAIN_TARGETS",
    ),
    (Opcode::SMSG_SET_PROFICIENCY, "SMSG_SET_PROFICIENCY"),
    (Opcode::SMSG_INITIAL_SPELLS, "SMSG_INITIAL_SPELLS"),
    (Opcode::SMSG_LEARNED_SPELL, "SMSG_LEARNED_SPELL"),
    (Opcode::SMSG_REMOVED_SPELL, "SMSG_REMOVED_SPELL"),
    (Opcode::SMSG_SPELL_FAILURE, "SMSG_SPELL_FAILURE"),
    (Opcode::SMSG_CLEAR_COOLDOWN, "SMSG_CLEAR_COOLDOWN"),
    (Opcode::SMSG_AURA_UPDATE, "SMSG_AURA_UPDATE"),
    (Opcode::SMSG_AURA_UPDATE_ALL, "SMSG_AURA_UPDATE_ALL"),
    (
        Opcode::SMSG_UPDATE_AURA_DURATION,
        "SMSG_UPDATE_AURA_DURATION",
    ),
    (Opcode::SMSG_SET_EXTRA_AURA_INFO, "SMSG_SET_EXTRA_AURA_INFO"),
    (Opcode::SMSG_PERIODICAURALOG, "SMSG_PERIODICAURALOG"),
    (Opcode::SMSG_SPELLDAMAGELOG, "SMSG_SPELLDAMAGELOG"),
    (Opcode::SMSG_SPELLHEALLOG, "SMSG_SPELLHEALLOG"),
    (Opcode::SMSG_SPELLLOGMISS, "SMSG_SPELLLOGMISS"),
    (Opcode::SMSG_SPELLENERGIZELOG, "SMSG_SPELLENERGIZELOG"),
    (
        Opcode::SMSG_SPELLNONMELEEDAMAGELOG,
        "SMSG_SPELLNONMELEEDAMAGELOG",
    ),
    (Opcode::SMSG_SPELLLOGEXECUTE, "SMSG_SPELLLOGEXECUTE"),
    (Opcode::SMSG_SPELLINSTAKILLLOG, "SMSG_SPELLINSTAKILLLOG"),
    (Opcode::SMSG_PROCRESIST, "SMSG_PROCRESIST"),
    (
        Opcode::SMSG_SPELLORDAMAGE_IMMUNE,
        "SMSG_SPELLORDAMAGE_IMMUNE",
    ),
    (Opcode::CMSG_SET_ACTION_BUTTON, "CMSG_SET_ACTION_BUTTON"),
    (Opcode::SMSG_ACTION_BUTTONS, "SMSG_ACTION_BUTTONS"),
    (
        Opcode::SMSG_DURABILITY_DAMAGE_DEATH,
        "SMSG_DURABILITY_DAMAGE_DEATH",
    ),
    (
        Opcode::SMSG_CORPSE_RECLAIM_DELAY,
        "SMSG_CORPSE_RECLAIM_DELAY",
    ),
    (Opcode::CMSG_REPOP_REQUEST, "CMSG_REPOP_REQUEST"),
    (Opcode::CMSG_RESURRECT_RESPONSE, "CMSG_RESURRECT_RESPONSE"),
    (Opcode::CMSG_RECLAIM_CORPSE, "CMSG_RECLAIM_CORPSE"),
    (Opcode::SMSG_RESURRECT_REQUEST, "SMSG_RESURRECT_REQUEST"),
    (
        Opcode::SMSG_SPIRIT_HEALER_CONFIRM,
        "SMSG_SPIRIT_HEALER_CONFIRM",
    ),
    (
        Opcode::CMSG_SPIRIT_HEALER_ACTIVATE,
        "CMSG_SPIRIT_HEALER_ACTIVATE",
    ),
    (Opcode::CMSG_SELF_RES, "CMSG_SELF_RES"),
    (Opcode::CMSG_GOSSIP_HELLO, "CMSG_GOSSIP_HELLO"),
    (
        Opcode::CMSG_GOSSIP_SELECT_OPTION,
        "CMSG_GOSSIP_SELECT_OPTION",
    ),
    (Opcode::SMSG_GOSSIP_MESSAGE, "SMSG_GOSSIP_MESSAGE"),
    (Opcode::SMSG_GOSSIP_COMPLETE, "SMSG_GOSSIP_COMPLETE"),
    (Opcode::SMSG_GOSSIP_POI, "SMSG_GOSSIP_POI"),
    (Opcode::SMSG_NPC_TEXT_UPDATE, "SMSG_NPC_TEXT_UPDATE"),
    (Opcode::CMSG_NPC_TEXT_QUERY, "CMSG_NPC_TEXT_QUERY"),
    (Opcode::CMSG_LIST_INVENTORY, "CMSG_LIST_INVENTORY"),
    (Opcode::SMSG_LIST_INVENTORY, "SMSG_LIST_INVENTORY"),
    (Opcode::CMSG_SELL_ITEM, "CMSG_SELL_ITEM"),
    (Opcode::SMSG_SELL_ITEM, "SMSG_SELL_ITEM"),
    (Opcode::CMSG_BUY_ITEM, "CMSG_BUY_ITEM"),
    (Opcode::CMSG_BUY_ITEM_IN_SLOT, "CMSG_BUY_ITEM_IN_SLOT"),
    (Opcode::SMSG_BUY_ITEM, "SMSG_BUY_ITEM"),
    (Opcode::SMSG_BUY_FAILED, "SMSG_BUY_FAILED"),
    (Opcode::SMSG_ITEM_PUSH_RESULT, "SMSG_ITEM_PUSH_RESULT"),
    (Opcode::CMSG_BUYBACK_ITEM, "CMSG_BUYBACK_ITEM"),
    (Opcode::CMSG_TRAINER_LIST, "CMSG_TRAINER_LIST"),
    (Opcode::SMSG_TRAINER_LIST, "SMSG_TRAINER_LIST"),
    (Opcode::CMSG_TRAINER_BUY_SPELL, "CMSG_TRAINER_BUY_SPELL"),
    (
        Opcode::SMSG_TRAINER_BUY_SUCCEEDED,
        "SMSG_TRAINER_BUY_SUCCEEDED",
    ),
    (Opcode::SMSG_TRAINER_BUY_FAILED, "SMSG_TRAINER_BUY_FAILED"),
    (Opcode::CMSG_BANKER_ACTIVATE, "CMSG_BANKER_ACTIVATE"),
    (Opcode::SMSG_SHOW_BANK, "SMSG_SHOW_BANK"),
    (Opcode::CMSG_BUY_BANK_SLOT, "CMSG_BUY_BANK_SLOT"),
    (
        Opcode::SMSG_BUY_BANK_SLOT_RESULT,
        "SMSG_BUY_BANK_SLOT_RESULT",
    ),
    (Opcode::CMSG_AUTOBANK_ITEM, "CMSG_AUTOBANK_ITEM"),
    (Opcode::CMSG_AUTOSTORE_BANK_ITEM, "CMSG_AUTOSTORE_BANK_ITEM"),
    (Opcode::CMSG_BINDER_ACTIVATE, "CMSG_BINDER_ACTIVATE"),
    (
        Opcode::MSG_TABARDVENDOR_ACTIVATE,
        "MSG_TABARDVENDOR_ACTIVATE",
    ),
    (
        Opcode::CMSG_TAXINODE_STATUS_QUERY,
        "CMSG_TAXINODE_STATUS_QUERY",
    ),
    (Opcode::SMSG_TAXINODE_STATUS, "SMSG_TAXINODE_STATUS"),
    (
        Opcode::CMSG_TAXIQUERYAVAILABLENODES,
        "CMSG_TAXIQUERYAVAILABLENODES",
    ),
    (Opcode::SMSG_SHOWTAXINODES, "SMSG_SHOWTAXINODES"),
    (Opcode::CMSG_ACTIVATETAXI, "CMSG_ACTIVATETAXI"),
    (Opcode::SMSG_ACTIVATETAXIREPLY, "SMSG_ACTIVATETAXIREPLY"),
    (Opcode::SMSG_NEW_TAXI_PATH, "SMSG_NEW_TAXI_PATH"),
    (Opcode::CMSG_LEARN_TALENT, "CMSG_LEARN_TALENT"),
    (Opcode::CMSG_UNLEARN_TALENTS, "CMSG_UNLEARN_TALENTS"),
    (Opcode::CMSG_UNLEARN_SPELL, "CMSG_UNLEARN_SPELL"),
    (Opcode::CMSG_UNLEARN_SKILL, "CMSG_UNLEARN_SKILL"),
    (Opcode::SMSG_BINDPOINTUPDATE, "SMSG_BINDPOINTUPDATE"),
    (Opcode::SMSG_BINDZONEREPLY, "SMSG_BINDZONEREPLY"),
    (Opcode::SMSG_PLAYERBOUND, "SMSG_PLAYERBOUND"),
    (Opcode::CMSG_SETDEATHBINDPOINT, "CMSG_SETDEATHBINDPOINT"),
    (Opcode::CMSG_GETDEATHBINDZONE, "CMSG_GETDEATHBINDZONE"),
    (Opcode::SMSG_SET_REST_START, "SMSG_SET_REST_START"),
    (Opcode::SMSG_LOG_XPGAIN, "SMSG_LOG_XPGAIN"),
    (Opcode::SMSG_LEVELUP_INFO, "SMSG_LEVELUP_INFO"),
    (Opcode::SMSG_START_MIRROR_TIMER, "SMSG_START_MIRROR_TIMER"),
    (Opcode::SMSG_STOP_MIRROR_TIMER, "SMSG_STOP_MIRROR_TIMER"),
    (
        Opcode::SMSG_ENVIRONMENTALDAMAGELOG,
        "SMSG_ENVIRONMENTALDAMAGELOG",
    ),
    (
        Opcode::SMSG_EXPLORATION_EXPERIENCE,
        "SMSG_EXPLORATION_EXPERIENCE",
    ),
    (Opcode::SMSG_INIT_WORLD_STATES, "SMSG_INIT_WORLD_STATES"),
    (Opcode::SMSG_INITIALIZE_FACTIONS, "SMSG_INITIALIZE_FACTIONS"),
    (
        Opcode::SMSG_SET_FACTION_STANDING,
        "SMSG_SET_FACTION_STANDING",
    ),
    (Opcode::SMSG_SET_FACTION_VISIBLE, "SMSG_SET_FACTION_VISIBLE"),
    (Opcode::CMSG_SET_FACTION_ATWAR, "CMSG_SET_FACTION_ATWAR"),
    (
        Opcode::CMSG_SET_FACTION_INACTIVE,
        "CMSG_SET_FACTION_INACTIVE",
    ),
    (Opcode::SMSG_TRIGGER_CINEMATIC, "SMSG_TRIGGER_CINEMATIC"),
    (
        Opcode::CMSG_NEXT_CINEMATIC_CAMERA,
        "CMSG_NEXT_CINEMATIC_CAMERA",
    ),
    (Opcode::CMSG_COMPLETE_CINEMATIC, "CMSG_COMPLETE_CINEMATIC"),
    (
        Opcode::CMSG_SET_ACTION_BAR_TOGGLES,
        "CMSG_SET_ACTION_BAR_TOGGLES",
    ),
    (Opcode::CMSG_ZONEUPDATE, "CMSG_ZONEUPDATE"),
    (Opcode::CMSG_OPEN_ITEM, "CMSG_OPEN_ITEM"),
    (Opcode::CMSG_READ_ITEM, "CMSG_READ_ITEM"),
    (Opcode::SMSG_READ_ITEM_OK, "SMSG_READ_ITEM_OK"),
    (Opcode::SMSG_READ_ITEM_FAILED, "SMSG_READ_ITEM_FAILED"),
    (Opcode::SMSG_ITEM_COOLDOWN, "SMSG_ITEM_COOLDOWN"),
    (
        Opcode::SMSG_INVENTORY_CHANGE_FAILURE,
        "SMSG_INVENTORY_CHANGE_FAILURE",
    ),
    (Opcode::SMSG_OPEN_CONTAINER, "SMSG_OPEN_CONTAINER"),
    (
        Opcode::CMSG_AUTOEQUIP_GROUND_ITEM,
        "CMSG_AUTOEQUIP_GROUND_ITEM",
    ),
    (
        Opcode::CMSG_AUTOSTORE_GROUND_ITEM,
        "CMSG_AUTOSTORE_GROUND_ITEM",
    ),
    (Opcode::CMSG_AUTOSTORE_LOOT_ITEM, "CMSG_AUTOSTORE_LOOT_ITEM"),
    (Opcode::CMSG_STORE_LOOT_IN_SLOT, "CMSG_STORE_LOOT_IN_SLOT"),
    (Opcode::CMSG_AUTOEQUIP_ITEM, "CMSG_AUTOEQUIP_ITEM"),
    (Opcode::CMSG_AUTOSTORE_BAG_ITEM, "CMSG_AUTOSTORE_BAG_ITEM"),
    (Opcode::CMSG_SWAP_ITEM, "CMSG_SWAP_ITEM"),
    (Opcode::CMSG_SWAP_INV_ITEM, "CMSG_SWAP_INV_ITEM"),
    (Opcode::CMSG_SPLIT_ITEM, "CMSG_SPLIT_ITEM"),
    (Opcode::CMSG_AUTOEQUIP_ITEM_SLOT, "CMSG_AUTOEQUIP_ITEM_SLOT"),
    (Opcode::CMSG_DROP_ITEM, "CMSG_DROP_ITEM"),
    (Opcode::CMSG_DESTROYITEM, "CMSG_DESTROYITEM"),
    (Opcode::CMSG_INSPECT, "CMSG_INSPECT"),
    (Opcode::SMSG_INSPECT, "SMSG_INSPECT"),
    (Opcode::MSG_INSPECT_HONOR_STATS, "MSG_INSPECT_HONOR_STATS"),
    (Opcode::CMSG_REPAIR_ITEM, "CMSG_REPAIR_ITEM"),
    (Opcode::SMSG_ITEM_TIME_UPDATE, "SMSG_ITEM_TIME_UPDATE"),
    (
        Opcode::SMSG_ITEM_ENCHANT_TIME_UPDATE,
        "SMSG_ITEM_ENCHANT_TIME_UPDATE",
    ),
    (Opcode::CMSG_SET_AMMO, "CMSG_SET_AMMO"),
    (Opcode::CMSG_WRAP_ITEM, "CMSG_WRAP_ITEM"),
    (Opcode::CMSG_GAMEOBJ_USE, "CMSG_GAMEOBJ_USE"),
    (Opcode::CMSG_AREATRIGGER, "CMSG_AREATRIGGER"),
    (Opcode::CMSG_MESSAGECHAT, "CMSG_MESSAGECHAT"),
    (Opcode::SMSG_MESSAGECHAT, "SMSG_MESSAGECHAT"),
    (Opcode::CMSG_CHAT_IGNORED, "CMSG_CHAT_IGNORED"),
    (Opcode::SMSG_CHAT_WRONG_FACTION, "SMSG_CHAT_WRONG_FACTION"),
    (
        Opcode::SMSG_CHAT_PLAYER_NOT_FOUND,
        "SMSG_CHAT_PLAYER_NOT_FOUND",
    ),
    (Opcode::SMSG_CHAT_RESTRICTED, "SMSG_CHAT_RESTRICTED"),
    (
        Opcode::SMSG_CHAT_PLAYER_AMBIGUOUS,
        "SMSG_CHAT_PLAYER_AMBIGUOUS",
    ),
    (Opcode::CMSG_CHAT_FILTERED, "CMSG_CHAT_FILTERED"),
    (Opcode::CMSG_EMOTE, "CMSG_EMOTE"),
    (Opcode::CMSG_TEXT_EMOTE, "CMSG_TEXT_EMOTE"),
    (Opcode::SMSG_TEXT_EMOTE, "SMSG_TEXT_EMOTE"),
    (Opcode::SMSG_EMOTE, "SMSG_EMOTE"),
    (Opcode::SMSG_PLAY_OBJECT_SOUND, "SMSG_PLAY_OBJECT_SOUND"),
    (Opcode::SMSG_PLAY_SOUND, "SMSG_PLAY_SOUND"),
    (Opcode::SMSG_PLAY_SPELL_VISUAL, "SMSG_PLAY_SPELL_VISUAL"),
    (Opcode::CMSG_JOIN_CHANNEL, "CMSG_JOIN_CHANNEL"),
    (Opcode::CMSG_LEAVE_CHANNEL, "CMSG_LEAVE_CHANNEL"),
    (Opcode::SMSG_CHANNEL_NOTIFY, "SMSG_CHANNEL_NOTIFY"),
    (Opcode::CMSG_CHANNEL_LIST, "CMSG_CHANNEL_LIST"),
    (Opcode::SMSG_CHANNEL_LIST, "SMSG_CHANNEL_LIST"),
    (Opcode::CMSG_CHANNEL_PASSWORD, "CMSG_CHANNEL_PASSWORD"),
    (Opcode::CMSG_CHANNEL_SET_OWNER, "CMSG_CHANNEL_SET_OWNER"),
    (Opcode::CMSG_CHANNEL_OWNER, "CMSG_CHANNEL_OWNER"),
    (Opcode::CMSG_CHANNEL_MODERATOR, "CMSG_CHANNEL_MODERATOR"),
    (Opcode::CMSG_CHANNEL_UNMODERATOR, "CMSG_CHANNEL_UNMODERATOR"),
    (Opcode::CMSG_CHANNEL_MUTE, "CMSG_CHANNEL_MUTE"),
    (Opcode::CMSG_CHANNEL_UNMUTE, "CMSG_CHANNEL_UNMUTE"),
    (Opcode::CMSG_CHANNEL_INVITE, "CMSG_CHANNEL_INVITE"),
    (Opcode::CMSG_CHANNEL_KICK, "CMSG_CHANNEL_KICK"),
    (Opcode::CMSG_CHANNEL_BAN, "CMSG_CHANNEL_BAN"),
    (Opcode::CMSG_CHANNEL_UNBAN, "CMSG_CHANNEL_UNBAN"),
    (
        Opcode::CMSG_CHANNEL_ANNOUNCEMENTS,
        "CMSG_CHANNEL_ANNOUNCEMENTS",
    ),
    (Opcode::CMSG_CHANNEL_MODERATE, "CMSG_CHANNEL_MODERATE"),
    (Opcode::CMSG_WHO, "CMSG_WHO"),
    (Opcode::SMSG_WHO, "SMSG_WHO"),
    (Opcode::CMSG_FRIEND_LIST, "CMSG_FRIEND_LIST"),
    (Opcode::SMSG_FRIEND_LIST, "SMSG_FRIEND_LIST"),
    (Opcode::SMSG_FRIEND_STATUS, "SMSG_FRIEND_STATUS"),
    (Opcode::CMSG_ADD_FRIEND, "CMSG_ADD_FRIEND"),
    (Opcode::CMSG_DEL_FRIEND, "CMSG_DEL_FRIEND"),
    (Opcode::SMSG_IGNORE_LIST, "SMSG_IGNORE_LIST"),
    (Opcode::CMSG_ADD_IGNORE, "CMSG_ADD_IGNORE"),
    (Opcode::CMSG_DEL_IGNORE, "CMSG_DEL_IGNORE"),
    (Opcode::CMSG_GROUP_INVITE, "CMSG_GROUP_INVITE"),
    (Opcode::SMSG_GROUP_INVITE, "SMSG_GROUP_INVITE"),
    (Opcode::MSG_PARTY_LEAVE, "MSG_PARTY_LEAVE"),
    (Opcode::CMSG_GROUP_ACCEPT, "CMSG_GROUP_ACCEPT"),
    (Opcode::CMSG_GROUP_DECLINE, "CMSG_GROUP_DECLINE"),
    (Opcode::SMSG_GROUP_DECLINE, "SMSG_GROUP_DECLINE"),
    (Opcode::CMSG_GROUP_UNINVITE, "CMSG_GROUP_UNINVITE"),
    (Opcode::SMSG_GROUP_UNINVITE, "SMSG_GROUP_UNINVITE"),
    (Opcode::CMSG_GROUP_SET_LEADER, "CMSG_GROUP_SET_LEADER"),
    (Opcode::SMSG_GROUP_SET_LEADER, "SMSG_GROUP_SET_LEADER"),
    (Opcode::CMSG_LOOT_METHOD, "CMSG_LOOT_METHOD"),
    (Opcode::CMSG_GROUP_DISBAND, "CMSG_GROUP_DISBAND"),
    (Opcode::SMSG_GROUP_DESTROYED, "SMSG_GROUP_DESTROYED"),
    (Opcode::SMSG_GROUP_LIST, "SMSG_GROUP_LIST"),
    (Opcode::SMSG_PARTY_MEMBER_STATS, "SMSG_PARTY_MEMBER_STATS"),
    (
        Opcode::SMSG_PARTY_COMMAND_RESULT,
        "SMSG_PARTY_COMMAND_RESULT",
    ),
    (
        Opcode::CMSG_GROUP_CHANGE_SUB_GROUP,
        "CMSG_GROUP_CHANGE_SUB_GROUP",
    ),
    (
        Opcode::CMSG_GROUP_SWAP_SUB_GROUP,
        "CMSG_GROUP_SWAP_SUB_GROUP",
    ),
    (
        Opcode::CMSG_GROUP_ASSISTANT_LEADER,
        "CMSG_GROUP_ASSISTANT_LEADER",
    ),
    (Opcode::CMSG_GROUP_RAID_CONVERT, "CMSG_GROUP_RAID_CONVERT"),
    (
        Opcode::CMSG_REQUEST_PARTY_MEMBER_STATS,
        "CMSG_REQUEST_PARTY_MEMBER_STATS",
    ),
    (
        Opcode::SMSG_PARTY_MEMBER_STATS_FULL,
        "SMSG_PARTY_MEMBER_STATS_FULL",
    ),
    (Opcode::MSG_RAID_TARGET_UPDATE, "MSG_RAID_TARGET_UPDATE"),
    (Opcode::MSG_RAID_READY_CHECK, "MSG_RAID_READY_CHECK"),
    (Opcode::MSG_MINIMAP_PING, "MSG_MINIMAP_PING"),
    (Opcode::MSG_RANDOM_ROLL, "MSG_RANDOM_ROLL"),
    (Opcode::CMSG_LOOT, "CMSG_LOOT"),
    (Opcode::CMSG_LOOT_MONEY, "CMSG_LOOT_MONEY"),
    (Opcode::CMSG_LOOT_RELEASE, "CMSG_LOOT_RELEASE"),
    (Opcode::SMSG_LOOT_RESPONSE, "SMSG_LOOT_RESPONSE"),
    (
        Opcode::SMSG_LOOT_RELEASE_RESPONSE,
        "SMSG_LOOT_RELEASE_RESPONSE",
    ),
    (Opcode::SMSG_LOOT_REMOVED, "SMSG_LOOT_REMOVED"),
    (Opcode::SMSG_LOOT_MONEY_NOTIFY, "SMSG_LOOT_MONEY_NOTIFY"),
    (Opcode::SMSG_LOOT_CLEAR_MONEY, "SMSG_LOOT_CLEAR_MONEY"),
    (Opcode::CMSG_LOOT_ROLL, "CMSG_LOOT_ROLL"),
    (Opcode::SMSG_LOOT_START_ROLL, "SMSG_LOOT_START_ROLL"),
    (Opcode::SMSG_LOOT_ROLL, "SMSG_LOOT_ROLL"),
    (Opcode::CMSG_LOOT_MASTER_GIVE, "CMSG_LOOT_MASTER_GIVE"),
    (Opcode::SMSG_LOOT_MASTER_LIST, "SMSG_LOOT_MASTER_LIST"),
    (Opcode::SMSG_LOOT_ROLL_WON, "SMSG_LOOT_ROLL_WON"),
    (Opcode::SMSG_LOOT_ALL_PASSED, "SMSG_LOOT_ALL_PASSED"),
    (Opcode::CMSG_INITIATE_TRADE, "CMSG_INITIATE_TRADE"),
    (Opcode::CMSG_BEGIN_TRADE, "CMSG_BEGIN_TRADE"),
    (Opcode::CMSG_BUSY_TRADE, "CMSG_BUSY_TRADE"),
    (Opcode::CMSG_IGNORE_TRADE, "CMSG_IGNORE_TRADE"),
    (Opcode::CMSG_ACCEPT_TRADE, "CMSG_ACCEPT_TRADE"),
    (Opcode::CMSG_UNACCEPT_TRADE, "CMSG_UNACCEPT_TRADE"),
    (Opcode::CMSG_CANCEL_TRADE, "CMSG_CANCEL_TRADE"),
    (Opcode::CMSG_SET_TRADE_ITEM, "CMSG_SET_TRADE_ITEM"),
    (Opcode::CMSG_CLEAR_TRADE_ITEM, "CMSG_CLEAR_TRADE_ITEM"),
    (Opcode::CMSG_SET_TRADE_GOLD, "CMSG_SET_TRADE_GOLD"),
    (Opcode::SMSG_TRADE_STATUS, "SMSG_TRADE_STATUS"),
    (
        Opcode::SMSG_TRADE_STATUS_EXTENDED,
        "SMSG_TRADE_STATUS_EXTENDED",
    ),
    (Opcode::CMSG_QUEST_QUERY, "CMSG_QUEST_QUERY"),
    (
        Opcode::SMSG_QUEST_QUERY_RESPONSE,
        "SMSG_QUEST_QUERY_RESPONSE",
    ),
    (
        Opcode::CMSG_QUESTGIVER_STATUS_QUERY,
        "CMSG_QUESTGIVER_STATUS_QUERY",
    ),
    (Opcode::SMSG_QUESTGIVER_STATUS, "SMSG_QUESTGIVER_STATUS"),
    (
        Opcode::CMSG_QUESTGIVER_STATUS_MULTIPLE_QUERY,
        "CMSG_QUESTGIVER_STATUS_MULTIPLE_QUERY",
    ),
    (Opcode::CMSG_CLOSE_INTERACTION, "CMSG_CLOSE_INTERACTION"),
    (Opcode::SMSG_TIME_SYNC_REQUEST, "SMSG_TIME_SYNC_REQUEST"),
    (Opcode::CMSG_TIME_SYNC_RESPONSE, "CMSG_TIME_SYNC_RESPONSE"),
    (
        Opcode::CMSG_TIME_SYNC_RESPONSE_FAILED,
        "CMSG_TIME_SYNC_RESPONSE_FAILED",
    ),
    (
        Opcode::CMSG_TIME_SYNC_RESPONSE_DROPPED,
        "CMSG_TIME_SYNC_RESPONSE_DROPPED",
    ),
    (
        Opcode::SMSG_QUESTGIVER_STATUS_MULTIPLE,
        "SMSG_QUESTGIVER_STATUS_MULTIPLE",
    ),
    (Opcode::CMSG_QUESTGIVER_HELLO, "CMSG_QUESTGIVER_HELLO"),
    (
        Opcode::SMSG_QUESTGIVER_QUEST_LIST,
        "SMSG_QUESTGIVER_QUEST_LIST",
    ),
    (
        Opcode::CMSG_QUESTGIVER_QUERY_QUEST,
        "CMSG_QUESTGIVER_QUERY_QUEST",
    ),
    (
        Opcode::CMSG_QUESTGIVER_QUEST_AUTOLAUNCH,
        "CMSG_QUESTGIVER_QUEST_AUTOLAUNCH",
    ),
    (
        Opcode::SMSG_QUESTGIVER_QUEST_DETAILS,
        "SMSG_QUESTGIVER_QUEST_DETAILS",
    ),
    (
        Opcode::CMSG_QUESTGIVER_ACCEPT_QUEST,
        "CMSG_QUESTGIVER_ACCEPT_QUEST",
    ),
    (
        Opcode::CMSG_QUESTGIVER_COMPLETE_QUEST,
        "CMSG_QUESTGIVER_COMPLETE_QUEST",
    ),
    (
        Opcode::SMSG_QUESTGIVER_REQUEST_ITEMS,
        "SMSG_QUESTGIVER_REQUEST_ITEMS",
    ),
    (
        Opcode::CMSG_QUESTGIVER_REQUEST_REWARD,
        "CMSG_QUESTGIVER_REQUEST_REWARD",
    ),
    (
        Opcode::SMSG_QUESTGIVER_OFFER_REWARD,
        "SMSG_QUESTGIVER_OFFER_REWARD",
    ),
    (
        Opcode::CMSG_QUESTGIVER_CHOOSE_REWARD,
        "CMSG_QUESTGIVER_CHOOSE_REWARD",
    ),
    (
        Opcode::SMSG_QUESTGIVER_QUEST_INVALID,
        "SMSG_QUESTGIVER_QUEST_INVALID",
    ),
    (Opcode::CMSG_QUESTGIVER_CANCEL, "CMSG_QUESTGIVER_CANCEL"),
    (
        Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE,
        "SMSG_QUESTGIVER_QUEST_COMPLETE",
    ),
    (
        Opcode::SMSG_QUESTGIVER_QUEST_FAILED,
        "SMSG_QUESTGIVER_QUEST_FAILED",
    ),
    (Opcode::CMSG_QUESTLOG_SWAP_QUEST, "CMSG_QUESTLOG_SWAP_QUEST"),
    (
        Opcode::CMSG_QUESTLOG_REMOVE_QUEST,
        "CMSG_QUESTLOG_REMOVE_QUEST",
    ),
    (Opcode::SMSG_QUESTLOG_FULL, "SMSG_QUESTLOG_FULL"),
    (Opcode::SMSG_QUESTUPDATE_FAILED, "SMSG_QUESTUPDATE_FAILED"),
    (
        Opcode::SMSG_QUESTUPDATE_FAILEDTIMER,
        "SMSG_QUESTUPDATE_FAILEDTIMER",
    ),
    (
        Opcode::SMSG_QUESTUPDATE_COMPLETE,
        "SMSG_QUESTUPDATE_COMPLETE",
    ),
    (
        Opcode::SMSG_QUESTUPDATE_ADD_KILL,
        "SMSG_QUESTUPDATE_ADD_KILL",
    ),
    (
        Opcode::SMSG_QUESTUPDATE_ADD_ITEM,
        "SMSG_QUESTUPDATE_ADD_ITEM",
    ),
    (
        Opcode::CMSG_QUEST_CONFIRM_ACCEPT,
        "CMSG_QUEST_CONFIRM_ACCEPT",
    ),
    (
        Opcode::SMSG_QUEST_CONFIRM_ACCEPT,
        "SMSG_QUEST_CONFIRM_ACCEPT",
    ),
    (Opcode::CMSG_PUSHQUESTTOPARTY, "CMSG_PUSHQUESTTOPARTY"),
    (Opcode::MSG_QUEST_PUSH_RESULT, "MSG_QUEST_PUSH_RESULT"),
    (Opcode::CMSG_GUILD_QUERY, "CMSG_GUILD_QUERY"),
    (
        Opcode::SMSG_GUILD_QUERY_RESPONSE,
        "SMSG_GUILD_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_GUILD_CREATE, "CMSG_GUILD_CREATE"),
    (Opcode::CMSG_GUILD_INVITE, "CMSG_GUILD_INVITE"),
    (Opcode::SMSG_GUILD_INVITE, "SMSG_GUILD_INVITE"),
    (Opcode::CMSG_GUILD_ACCEPT, "CMSG_GUILD_ACCEPT"),
    (Opcode::CMSG_GUILD_DECLINE, "CMSG_GUILD_DECLINE"),
    (Opcode::SMSG_GUILD_DECLINE, "SMSG_GUILD_DECLINE"),
    (Opcode::CMSG_GUILD_INFO, "CMSG_GUILD_INFO"),
    (Opcode::SMSG_GUILD_INFO, "SMSG_GUILD_INFO"),
    (Opcode::CMSG_GUILD_ROSTER, "CMSG_GUILD_ROSTER"),
    (Opcode::SMSG_GUILD_ROSTER, "SMSG_GUILD_ROSTER"),
    (Opcode::CMSG_GUILD_PROMOTE, "CMSG_GUILD_PROMOTE"),
    (Opcode::CMSG_GUILD_DEMOTE, "CMSG_GUILD_DEMOTE"),
    (Opcode::CMSG_GUILD_LEAVE, "CMSG_GUILD_LEAVE"),
    (Opcode::CMSG_GUILD_REMOVE, "CMSG_GUILD_REMOVE"),
    (Opcode::CMSG_GUILD_DISBAND, "CMSG_GUILD_DISBAND"),
    (Opcode::CMSG_GUILD_LEADER, "CMSG_GUILD_LEADER"),
    (Opcode::CMSG_GUILD_MOTD, "CMSG_GUILD_MOTD"),
    (Opcode::SMSG_GUILD_EVENT, "SMSG_GUILD_EVENT"),
    (
        Opcode::SMSG_GUILD_COMMAND_RESULT,
        "SMSG_GUILD_COMMAND_RESULT",
    ),
    (Opcode::CMSG_GUILD_RANK, "CMSG_GUILD_RANK"),
    (Opcode::CMSG_GUILD_ADD_RANK, "CMSG_GUILD_ADD_RANK"),
    (Opcode::CMSG_GUILD_DEL_RANK, "CMSG_GUILD_DEL_RANK"),
    (
        Opcode::CMSG_GUILD_SET_PUBLIC_NOTE,
        "CMSG_GUILD_SET_PUBLIC_NOTE",
    ),
    (
        Opcode::CMSG_GUILD_SET_OFFICER_NOTE,
        "CMSG_GUILD_SET_OFFICER_NOTE",
    ),
    (Opcode::CMSG_GUILD_INFO_TEXT, "CMSG_GUILD_INFO_TEXT"),
    (Opcode::MSG_SAVE_GUILD_EMBLEM, "MSG_SAVE_GUILD_EMBLEM"),
    (Opcode::CMSG_PETITION_SHOWLIST, "CMSG_PETITION_SHOWLIST"),
    (Opcode::SMSG_PETITION_SHOWLIST, "SMSG_PETITION_SHOWLIST"),
    (Opcode::CMSG_PETITION_BUY, "CMSG_PETITION_BUY"),
    (
        Opcode::CMSG_PETITION_SHOW_SIGNATURES,
        "CMSG_PETITION_SHOW_SIGNATURES",
    ),
    (
        Opcode::SMSG_PETITION_SHOW_SIGNATURES,
        "SMSG_PETITION_SHOW_SIGNATURES",
    ),
    (Opcode::CMSG_PETITION_SIGN, "CMSG_PETITION_SIGN"),
    (
        Opcode::SMSG_PETITION_SIGN_RESULTS,
        "SMSG_PETITION_SIGN_RESULTS",
    ),
    (Opcode::MSG_PETITION_DECLINE, "MSG_PETITION_DECLINE"),
    (
        Opcode::SMSG_PETITION_QUERY_RESPONSE,
        "SMSG_PETITION_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_TURN_IN_PETITION, "CMSG_TURN_IN_PETITION"),
    (
        Opcode::SMSG_TURN_IN_PETITION_RESULTS,
        "SMSG_TURN_IN_PETITION_RESULTS",
    ),
    (Opcode::CMSG_OFFER_PETITION, "CMSG_OFFER_PETITION"),
    (Opcode::MSG_PETITION_RENAME, "MSG_PETITION_RENAME"),
    (Opcode::MSG_QUERY_NEXT_MAIL_TIME, "MSG_QUERY_NEXT_MAIL_TIME"),
    (Opcode::CMSG_SEND_MAIL, "CMSG_SEND_MAIL"),
    (Opcode::SMSG_SEND_MAIL_RESULT, "SMSG_SEND_MAIL_RESULT"),
    (Opcode::CMSG_GET_MAIL_LIST, "CMSG_GET_MAIL_LIST"),
    (Opcode::SMSG_MAIL_LIST_RESULT, "SMSG_MAIL_LIST_RESULT"),
    (Opcode::CMSG_MAIL_TAKE_MONEY, "CMSG_MAIL_TAKE_MONEY"),
    (Opcode::CMSG_MAIL_TAKE_ITEM, "CMSG_MAIL_TAKE_ITEM"),
    (Opcode::CMSG_MAIL_MARK_AS_READ, "CMSG_MAIL_MARK_AS_READ"),
    (
        Opcode::CMSG_MAIL_RETURN_TO_SENDER,
        "CMSG_MAIL_RETURN_TO_SENDER",
    ),
    (Opcode::CMSG_MAIL_DELETE, "CMSG_MAIL_DELETE"),
    (
        Opcode::CMSG_MAIL_CREATE_TEXT_ITEM,
        "CMSG_MAIL_CREATE_TEXT_ITEM",
    ),
    (Opcode::SMSG_RECEIVED_MAIL, "SMSG_RECEIVED_MAIL"),
    (Opcode::MSG_AUCTION_HELLO, "MSG_AUCTION_HELLO"),
    (Opcode::CMSG_AUCTION_SELL_ITEM, "CMSG_AUCTION_SELL_ITEM"),
    (Opcode::CMSG_AUCTION_REMOVE_ITEM, "CMSG_AUCTION_REMOVE_ITEM"),
    (Opcode::CMSG_AUCTION_LIST_ITEMS, "CMSG_AUCTION_LIST_ITEMS"),
    (
        Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS,
        "CMSG_AUCTION_LIST_OWNER_ITEMS",
    ),
    (Opcode::CMSG_AUCTION_PLACE_BID, "CMSG_AUCTION_PLACE_BID"),
    (
        Opcode::SMSG_AUCTION_COMMAND_RESULT,
        "SMSG_AUCTION_COMMAND_RESULT",
    ),
    (Opcode::SMSG_AUCTION_LIST_RESULT, "SMSG_AUCTION_LIST_RESULT"),
    (
        Opcode::SMSG_AUCTION_OWNER_LIST_RESULT,
        "SMSG_AUCTION_OWNER_LIST_RESULT",
    ),
    (
        Opcode::SMSG_AUCTION_BIDDER_NOTIFICATION,
        "SMSG_AUCTION_BIDDER_NOTIFICATION",
    ),
    (
        Opcode::SMSG_AUCTION_OWNER_NOTIFICATION,
        "SMSG_AUCTION_OWNER_NOTIFICATION",
    ),
    (
        Opcode::CMSG_AUCTION_LIST_BIDDER_ITEMS,
        "CMSG_AUCTION_LIST_BIDDER_ITEMS",
    ),
    (
        Opcode::SMSG_AUCTION_BIDDER_LIST_RESULT,
        "SMSG_AUCTION_BIDDER_LIST_RESULT",
    ),
    (
        Opcode::SMSG_AUCTION_REMOVED_NOTIFICATION,
        "SMSG_AUCTION_REMOVED_NOTIFICATION",
    ),
    (Opcode::CMSG_BATTLEFIELD_STATUS, "CMSG_BATTLEFIELD_STATUS"),
    (Opcode::SMSG_BATTLEFIELD_STATUS, "SMSG_BATTLEFIELD_STATUS"),
    (Opcode::CMSG_BATTLEFIELD_LIST, "CMSG_BATTLEFIELD_LIST"),
    (Opcode::SMSG_BATTLEFIELD_LIST, "SMSG_BATTLEFIELD_LIST"),
    (Opcode::CMSG_BATTLEFIELD_JOIN, "CMSG_BATTLEFIELD_JOIN"),
    (Opcode::SMSG_BATTLEFIELD_JOINED, "SMSG_BATTLEFIELD_JOINED"),
    (Opcode::SMSG_BATTLEFIELD_LEFT, "SMSG_BATTLEFIELD_LEFT"),
    (Opcode::CMSG_LEAVE_BATTLEFIELD, "CMSG_LEAVE_BATTLEFIELD"),
    (Opcode::CMSG_BATTLEFIELD_PORT, "CMSG_BATTLEFIELD_PORT"),
    (Opcode::CMSG_BATTLEMASTER_HELLO, "CMSG_BATTLEMASTER_HELLO"),
    (Opcode::SMSG_BATTLEMASTER_JOINED, "SMSG_BATTLEMASTER_JOINED"),
    (Opcode::CMSG_BATTLEFIELD_QUEUE, "CMSG_BATTLEFIELD_QUEUE"),
    (
        Opcode::CMSG_BATTLEFIELD_UN_QUEUE,
        "CMSG_BATTLEFIELD_UN_QUEUE",
    ),
    (
        Opcode::CMSG_AREA_SPIRIT_HEALER_QUERY,
        "CMSG_AREA_SPIRIT_HEALER_QUERY",
    ),
    (
        Opcode::CMSG_AREA_SPIRIT_HEALER_QUEUE,
        "CMSG_AREA_SPIRIT_HEALER_QUEUE",
    ),
    (
        Opcode::SMSG_AREA_SPIRIT_HEALER_TIME,
        "SMSG_AREA_SPIRIT_HEALER_TIME",
    ),
    (Opcode::CMSG_REQUEST_RAID_INFO, "CMSG_REQUEST_RAID_INFO"),
    (Opcode::SMSG_RAID_INSTANCE_INFO, "SMSG_RAID_INSTANCE_INFO"),
    (Opcode::CMSG_RESET_INSTANCES, "CMSG_RESET_INSTANCES"),
    (Opcode::SMSG_INSTANCE_RESET, "SMSG_INSTANCE_RESET"),
    (
        Opcode::SMSG_INSTANCE_RESET_FAILED,
        "SMSG_INSTANCE_RESET_FAILED",
    ),
    (Opcode::CMSG_MEETINGSTONE_INFO, "CMSG_MEETINGSTONE_INFO"),
    (Opcode::CMSG_MEETINGSTONE_JOIN, "CMSG_MEETINGSTONE_JOIN"),
    (Opcode::CMSG_MEETINGSTONE_LEAVE, "CMSG_MEETINGSTONE_LEAVE"),
    (Opcode::CMSG_MEETINGSTONE_CHEAT, "CMSG_MEETINGSTONE_CHEAT"),
    (
        Opcode::SMSG_MEETINGSTONE_SETQUEUE,
        "SMSG_MEETINGSTONE_SETQUEUE",
    ),
    (Opcode::CMSG_DUEL_ACCEPTED, "CMSG_DUEL_ACCEPTED"),
    (Opcode::CMSG_DUEL_CANCELLED, "CMSG_DUEL_CANCELLED"),
    (Opcode::SMSG_DUEL_REQUESTED, "SMSG_DUEL_REQUESTED"),
    (Opcode::SMSG_DUEL_COUNTDOWN, "SMSG_DUEL_COUNTDOWN"),
    (Opcode::SMSG_DUEL_OUTOFBOUNDS, "SMSG_DUEL_OUTOFBOUNDS"),
    (Opcode::SMSG_DUEL_INBOUNDS, "SMSG_DUEL_INBOUNDS"),
    (Opcode::SMSG_DUEL_COMPLETE, "SMSG_DUEL_COMPLETE"),
    (Opcode::SMSG_DUEL_WINNER, "SMSG_DUEL_WINNER"),
    (Opcode::CMSG_PET_NAME_QUERY, "CMSG_PET_NAME_QUERY"),
    (
        Opcode::SMSG_PET_NAME_QUERY_RESPONSE,
        "SMSG_PET_NAME_QUERY_RESPONSE",
    ),
    (Opcode::CMSG_PET_SET_ACTION, "CMSG_PET_SET_ACTION"),
    (Opcode::CMSG_PET_ACTION, "CMSG_PET_ACTION"),
    (Opcode::CMSG_PET_ABANDON, "CMSG_PET_ABANDON"),
    (Opcode::CMSG_PET_RENAME, "CMSG_PET_RENAME"),
    (Opcode::SMSG_PET_SPELLS, "SMSG_PET_SPELLS"),
    (Opcode::SMSG_PET_MODE, "SMSG_PET_MODE"),
    (Opcode::SMSG_PET_TAME_FAILURE, "SMSG_PET_TAME_FAILURE"),
    (Opcode::SMSG_PET_NAME_INVALID, "SMSG_PET_NAME_INVALID"),
    (Opcode::CMSG_PET_CAST_SPELL, "CMSG_PET_CAST_SPELL"),
    (Opcode::SMSG_PET_CAST_FAILED, "SMSG_PET_CAST_FAILED"),
    (Opcode::CMSG_PET_CANCEL_AURA, "CMSG_PET_CANCEL_AURA"),
    (Opcode::SMSG_PET_ACTION_FEEDBACK, "SMSG_PET_ACTION_FEEDBACK"),
    (Opcode::SMSG_PET_BROKEN, "SMSG_PET_BROKEN"),
    (Opcode::CMSG_PET_UNLEARN, "CMSG_PET_UNLEARN"),
    (Opcode::SMSG_PET_UNLEARN_CONFIRM, "SMSG_PET_UNLEARN_CONFIRM"),
    (Opcode::CMSG_PET_SPELL_AUTOCAST, "CMSG_PET_SPELL_AUTOCAST"),
    (Opcode::CMSG_PET_STOP_ATTACK, "CMSG_PET_STOP_ATTACK"),
    (Opcode::CMSG_REQUEST_PET_INFO, "CMSG_REQUEST_PET_INFO"),
    (Opcode::MSG_LIST_STABLED_PETS, "MSG_LIST_STABLED_PETS"),
    (Opcode::CMSG_STABLE_PET, "CMSG_STABLE_PET"),
    (Opcode::CMSG_UNSTABLE_PET, "CMSG_UNSTABLE_PET"),
    (Opcode::CMSG_BUY_STABLE_SLOT, "CMSG_BUY_STABLE_SLOT"),
    (Opcode::SMSG_STABLE_RESULT, "SMSG_STABLE_RESULT"),
    (Opcode::CMSG_STABLE_REVIVE_PET, "CMSG_STABLE_REVIVE_PET"),
    (Opcode::CMSG_STABLE_SWAP_PET, "CMSG_STABLE_SWAP_PET"),
    (Opcode::CMSG_GMTICKET_CREATE, "CMSG_GMTICKET_CREATE"),
    (Opcode::SMSG_GMTICKET_CREATE, "SMSG_GMTICKET_CREATE"),
    (Opcode::CMSG_GMTICKET_UPDATETEXT, "CMSG_GMTICKET_UPDATETEXT"),
    (Opcode::SMSG_GMTICKET_UPDATETEXT, "SMSG_GMTICKET_UPDATETEXT"),
    (Opcode::CMSG_GMTICKET_GETTICKET, "CMSG_GMTICKET_GETTICKET"),
    (Opcode::SMSG_GMTICKET_GETTICKET, "SMSG_GMTICKET_GETTICKET"),
    (
        Opcode::CMSG_GMTICKET_DELETETICKET,
        "CMSG_GMTICKET_DELETETICKET",
    ),
    (
        Opcode::SMSG_GMTICKET_DELETETICKET,
        "SMSG_GMTICKET_DELETETICKET",
    ),
    (
        Opcode::CMSG_GMTICKET_SYSTEMSTATUS,
        "CMSG_GMTICKET_SYSTEMSTATUS",
    ),
    (
        Opcode::SMSG_GMTICKET_SYSTEMSTATUS,
        "SMSG_GMTICKET_SYSTEMSTATUS",
    ),
    (Opcode::CMSG_GMSURVEY_SUBMIT, "CMSG_GMSURVEY_SUBMIT"),
    (Opcode::CMSG_TOGGLE_PVP, "CMSG_TOGGLE_PVP"),
    (Opcode::SMSG_SUMMON_REQUEST, "SMSG_SUMMON_REQUEST"),
    (Opcode::CMSG_SUMMON_RESPONSE, "CMSG_SUMMON_RESPONSE"),
    (Opcode::CMSG_FAR_SIGHT, "CMSG_FAR_SIGHT"),
    (Opcode::CMSG_TOGGLE_HELM, "CMSG_TOGGLE_HELM"),
    (Opcode::CMSG_TOGGLE_CLOAK, "CMSG_TOGGLE_CLOAK"),
    (Opcode::CMSG_SAVE_PLAYER, "CMSG_SAVE_PLAYER"),
    (Opcode::CMSG_SETSHEATHED, "CMSG_SETSHEATHED"),
    (Opcode::CMSG_GHOST, "CMSG_GHOST"),
    (Opcode::CMSG_PLAYED_TIME, "CMSG_PLAYED_TIME"),
    (Opcode::SMSG_PLAYED_TIME, "SMSG_PLAYED_TIME"),
    (Opcode::CMSG_BUG, "CMSG_BUG"),
    (Opcode::CMSG_WARDEN_DATA, "CMSG_WARDEN_DATA"),
    (Opcode::SMSG_WARDEN_DATA, "SMSG_WARDEN_DATA"),
    (Opcode::SMSG_WEATHER, "SMSG_WEATHER"),
    (Opcode::SMSG_MOVE_UPDATE, "SMSG_MOVE_UPDATE"),
    (Opcode::SMSG_RESUME_COMMS, "SMSG_RESUME_COMMS"),
    (Opcode::SMSG_WORLD_SERVER_INFO, "SMSG_WORLD_SERVER_INFO"),
    (
        Opcode::SMSG_SET_ALL_TASK_PROGRESS,
        "SMSG_SET_ALL_TASK_PROGRESS",
    ),
    (Opcode::SMSG_INITIAL_SETUP, "SMSG_INITIAL_SETUP"),
    (Opcode::SMSG_LOAD_CUF_PROFILES, "SMSG_LOAD_CUF_PROFILES"),
    (Opcode::CMSG_CONNECT_TO_FAILED, "CMSG_CONNECT_TO_FAILED"),
    (
        Opcode::SMSG_ENTER_ENCRYPTED_MODE,
        "SMSG_ENTER_ENCRYPTED_MODE",
    ),
    (
        Opcode::CMSG_ENTER_ENCRYPTED_MODE_ACK,
        "CMSG_ENTER_ENCRYPTED_MODE_ACK",
    ),
    (
        Opcode::CMSG_AUTH_CONTINUED_SESSION,
        "CMSG_AUTH_CONTINUED_SESSION",
    ),
    (Opcode::SMSG_CONNECT_TO, "SMSG_CONNECT_TO"),
    (
        Opcode::CMSG_SERVER_TIME_OFFSET_REQUEST,
        "CMSG_SERVER_TIME_OFFSET_REQUEST",
    ),
    (Opcode::SMSG_SERVER_TIME_OFFSET, "SMSG_SERVER_TIME_OFFSET"),
    (
        Opcode::CMSG_UPDATE_VAS_PURCHASE_STATES,
        "CMSG_UPDATE_VAS_PURCHASE_STATES",
    ),
    (
        Opcode::CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS,
        "CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS",
    ),
    (
        Opcode::CMSG_BATTLE_PAY_GET_PRODUCT_LIST,
        "CMSG_BATTLE_PAY_GET_PRODUCT_LIST",
    ),
    (
        Opcode::CMSG_BATTLE_PAY_GET_PURCHASE_LIST,
        "CMSG_BATTLE_PAY_GET_PURCHASE_LIST",
    ),
    (Opcode::CMSG_LOG_DISCONNECT, "CMSG_LOG_DISCONNECT"),
    (Opcode::CMSG_QUEUED_MESSAGES_END, "CMSG_QUEUED_MESSAGES_END"),
    (
        Opcode::CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        "CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE",
    ),
    (
        Opcode::CMSG_REQUEST_CATEGORY_COOLDOWNS,
        "CMSG_REQUEST_CATEGORY_COOLDOWNS",
    ),
    (Opcode::SMSG_SEND_SPELL_HISTORY, "SMSG_SEND_SPELL_HISTORY"),
    (
        Opcode::CMSG_REQUEST_FORCED_REACTIONS,
        "CMSG_REQUEST_FORCED_REACTIONS",
    ),
    (
        Opcode::SMSG_SET_FORCED_REACTIONS,
        "SMSG_SET_FORCED_REACTIONS",
    ),
    (
        Opcode::CMSG_QUERY_NEXT_MAIL_TIME,
        "CMSG_QUERY_NEXT_MAIL_TIME",
    ),
    (
        Opcode::SMSG_MAIL_QUERY_NEXT_TIME_RESULT,
        "SMSG_MAIL_QUERY_NEXT_TIME_RESULT",
    ),
    (
        Opcode::CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS,
        "CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS",
    ),
    (
        Opcode::SMSG_CONQUEST_FORMULA_CONSTANTS,
        "SMSG_CONQUEST_FORMULA_CONSTANTS",
    ),
    (
        Opcode::CMSG_REQUEST_LFG_LIST_BLACKLIST,
        "CMSG_REQUEST_LFG_LIST_BLACKLIST",
    ),
    (
        Opcode::SMSG_LFG_LIST_UPDATE_BLACKLIST,
        "SMSG_LFG_LIST_UPDATE_BLACKLIST",
    ),
    (
        Opcode::CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY,
        "CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY",
    ),
    (
        Opcode::SMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY,
        "SMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY",
    ),
    (
        Opcode::CMSG_CALENDAR_GET_NUM_PENDING,
        "CMSG_CALENDAR_GET_NUM_PENDING",
    ),
    (
        Opcode::SMSG_CALENDAR_SEND_NUM_PENDING,
        "SMSG_CALENDAR_SEND_NUM_PENDING",
    ),
    (
        Opcode::CMSG_GET_ACCOUNT_CHARACTER_LIST,
        "CMSG_GET_ACCOUNT_CHARACTER_LIST",
    ),
    (
        Opcode::SMSG_GET_ACCOUNT_CHARACTER_LIST_RESULT,
        "SMSG_GET_ACCOUNT_CHARACTER_LIST_RESULT",
    ),
    (
        Opcode::CMSG_LOADING_SCREEN_NOTIFY,
        "CMSG_LOADING_SCREEN_NOTIFY",
    ),
    (
        Opcode::CMSG_CHAT_REGISTER_ADDON_PREFIXES,
        "CMSG_CHAT_REGISTER_ADDON_PREFIXES",
    ),
    (
        Opcode::CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES,
        "CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES",
    ),
    (Opcode::CMSG_VIOLENCE_LEVEL, "CMSG_VIOLENCE_LEVEL"),
    (
        Opcode::CMSG_QUERY_COUNTDOWN_TIMER,
        "CMSG_QUERY_COUNTDOWN_TIMER",
    ),
    (
        Opcode::CMSG_REQUEST_CEMETERY_LIST,
        "CMSG_REQUEST_CEMETERY_LIST",
    ),
    (
        Opcode::CMSG_REQUEST_BATTLEFIELD_STATUS,
        "CMSG_REQUEST_BATTLEFIELD_STATUS",
    ),
    (Opcode::CMSG_LFG_LIST_GET_STATUS, "CMSG_LFG_LIST_GET_STATUS"),
    (
        Opcode::CMSG_BATTLE_PET_REQUEST_JOURNAL,
        "CMSG_BATTLE_PET_REQUEST_JOURNAL",
    ),
    (Opcode::CMSG_ARENA_TEAM_ACCEPT, "CMSG_ARENA_TEAM_ACCEPT"),
    (
        Opcode::CMSG_GUILD_SET_ACHIEVEMENT_TRACKING,
        "CMSG_GUILD_SET_ACHIEVEMENT_TRACKING",
    ),
    (
        Opcode::CMSG_GM_TICKET_GET_CASE_STATUS,
        "CMSG_GM_TICKET_GET_CASE_STATUS",
    ),
];

/// Marks an empty slot in the wire index tables. `u16::MAX` is safe as a sentinel because `ALL` is
/// far shorter than that.
const NO_ENTRY: u16 = u16::MAX;

// Bounds of each protocol's numbering, used to size the direct-index tables. Values outside these
// ranges simply fail to resolve, which is the same answer an unknown opcode gets.
const VANILLA_LO: u32 = 0x0000;
const VANILLA_HI: u32 = 0x0D00;
const MODERN_LO: u32 = 0x2500;
const MODERN_HI: u32 = 0x3B00;

/// Wire number -> index into [`ALL`]. Built at compile time, so an inbound lookup is one bounds
/// check and two array reads rather than a scan of 577 constants per packet.
static VANILLA_INDEX: [u16; (VANILLA_HI - VANILLA_LO) as usize] = build_vanilla_index();
static MODERN_INDEX: [u16; (MODERN_HI - MODERN_LO) as usize] = build_modern_index();

const fn build_vanilla_index() -> [u16; (VANILLA_HI - VANILLA_LO) as usize] {
    let mut table = [NO_ENTRY; (VANILLA_HI - VANILLA_LO) as usize];
    let mut i = 0;
    while i < ALL.len() {
        let value = ALL[i].0.vanilla;
        if value >= VANILLA_LO && value < VANILLA_HI {
            let slot = (value - VANILLA_LO) as usize;
            // First declaration wins. Some vanilla values are currently declared twice; see the
            // duplicate allow-list in this file's tests.
            if table[slot] == NO_ENTRY {
                table[slot] = i as u16;
            }
        }
        i += 1;
    }
    table
}

const fn build_modern_index() -> [u16; (MODERN_HI - MODERN_LO) as usize] {
    let mut table = [NO_ENTRY; (MODERN_HI - MODERN_LO) as usize];
    let mut i = 0;
    while i < ALL.len() {
        let value = ALL[i].0.modern;
        if value >= MODERN_LO && value < MODERN_HI {
            let slot = (value - MODERN_LO) as usize;
            if table[slot] == NO_ENTRY {
                table[slot] = i as u16;
            }
        }
        i += 1;
    }
    table
}

fn lookup(table: &[u16], lo: u32, value: u32) -> Option<Opcode> {
    // `0` means "absent from this protocol" and is never a real opcode, so it must not resolve.
    if value == 0 || value < lo {
        return None;
    }
    let index = *table.get((value - lo) as usize)?;
    if index == NO_ENTRY {
        None
    } else {
        Some(ALL[index as usize].0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Opcode, ALL};

    /// Every `pub const NAME: Opcode = Opcode { … }` in this file, parsed from its own source.
    ///
    /// Reading the text rather than trusting [`ALL`] is the point: it is what lets the test below
    /// catch a constant that was added without a matching `ALL` entry.
    fn declared_opcodes() -> Vec<(&'static str, u32)> {
        let src = include_str!("opcodes.rs");
        let mut out = Vec::new();
        for line in src.lines() {
            let trimmed = line.trim();
            // Skip the worked examples in this module's own doc comment.
            if trimmed.starts_with("//") {
                continue;
            }
            let Some(rest) = trimmed.strip_prefix("pub const ") else {
                continue;
            };
            let Some((name, body)) = rest.split_once(": Opcode = Opcode {") else {
                continue;
            };
            if name == "NONE" {
                continue;
            }
            // A constant may declare either column or both, so parse the field rather than
            // assuming a position. Modern-only opcodes have no `vanilla:` at all.
            let vanilla = body
                .split_once("vanilla: 0x")
                .and_then(|(_, rest)| rest.split_once(','))
                .and_then(|(hex, _)| u32::from_str_radix(hex.trim(), 16).ok())
                .unwrap_or(0);
            out.push((name, vanilla));
        }
        out
    }

    /// Opcode values shared by more than one constant.
    ///
    /// Two of these are harmless aliases; the rest are **wrong values** that happen not to have
    /// caused a visible failure yet. They are allow-listed so this guard can go in without a
    /// behaviour change, and each is individually testable against a live 1.12 client — fix them
    /// one per commit, deleting the entry as you go. Correct values are HermesProxy's
    /// `World/Enums/V1_12_1_5875/Opcode.cs`.
    ///
    /// | value | constants | verdict |
    /// |---|---|---|
    /// | `0x013B` | `CMSG_CANCEL_CHANNELING`, `CMSG_CANCEL_CHANNELLING` | alias — one spelling should go |
    /// | `0x02CE` | `CMSG_MOVE_TIME_SKIPPED`, `MSG_MOVE_TIME_SKIPPED` | alias — same opcode, two names |
    /// | `0x0057` | `SMSG_ITEM_QUERY_MULTIPLE_RESPONSE` | wrong, should be `0x0059` |
    /// | `0x0148` | `SMSG_SPELLNONMELEEDAMAGELOG` | wrong, absent from the 1.12 table |
    /// | `0x01B5` | `CMSG_BANKER_ACTIVATE` | wrong; `0x01B5` belongs to `CMSG_BINDER_ACTIVATE` |
    /// | `0x01EC` | `SMSG_ITEM_ENCHANT_TIME_UPDATE` | wrong, should be `0x01EB` (collides with `SMSG_AUTH_CHALLENGE`) |
    /// | `0x0209` | `SMSG_ACCOUNT_DATA_MD5` | not a 1.12 opcode; `0x0209` is `SMSG_ACCOUNT_DATA_TIMES` |
    /// | `0x0216` | `SMSG_BUY_BANK_SLOT_RESULT` | wrong, should be `0x01BA` |
    /// | `0x023B` | `CMSG_BATTLEFIELD_LIST` | wrong, should be `0x023C` |
    /// | `0x02B3` | `SMSG_COMPRESSED_MOVES` | wrong, should be `0x02FB` — **live**, used by `updates/movement_data.rs` |
    /// | `0x02B3` | `SMSG_PET_BROKEN` | wrong, should be `0x02AF` |
    /// | `0x02E2` | `SMSG_BATTLEFIELD_LEFT` | not a 1.12 opcode; `0x02E2` is `CMSG_AREA_SPIRIT_HEALER_QUERY` |
    /// | `0x02E3` | `SMSG_BATTLEMASTER_JOINED` | not a 1.12 opcode; `0x02E3` is `CMSG_AREA_SPIRIT_HEALER_QUEUE` |
    ///
    /// Separately wrong but *not* duplicated, so not caught here: `SMSG_PONG` is `0x01D` and should
    /// be `0x01DD` (**live**, used by `messages/movement.rs`), and `SMSG_ENVIRONMENTALDAMAGELOG` is
    /// `0x0C1F`, far outside the 1.12 range.
    const KNOWN_DUPLICATE_VALUES: &[u32] = &[
        0x0057, 0x013B, 0x0148, 0x01B5, 0x01EC, 0x0209, 0x0216, 0x023B, 0x02B3, 0x02CE, 0x02E2,
        0x02E3,
    ];

    #[test]
    fn every_opcode_constant_is_parsed() {
        // Guards the parser above, not the data: if this file's declaration style changes, the
        // duplicate check would silently start passing on an empty set.
        assert_eq!(
            declared_opcodes().len(),
            631,
            "expected 631 opcode constants; update this count deliberately when adding opcodes"
        );
    }

    #[test]
    fn no_unexpected_duplicate_opcode_values() {
        let mut by_value: HashMap<u32, Vec<&str>> = HashMap::new();
        for (name, value) in declared_opcodes() {
            // 0 means "no vanilla number", which the modern-only constants all share.
            if value != 0 {
                by_value.entry(value).or_default().push(name);
            }
        }

        let mut unexpected: Vec<String> = by_value
            .iter()
            .filter(|(value, names)| names.len() > 1 && !KNOWN_DUPLICATE_VALUES.contains(value))
            .map(|(value, names)| format!("0x{value:04X}: {}", names.join(", ")))
            .collect();
        unexpected.sort();

        assert!(
            unexpected.is_empty(),
            "new duplicate opcode values introduced:\n  {}",
            unexpected.join("\n  ")
        );
    }

    #[test]
    fn the_duplicate_allow_list_has_no_stale_entries() {
        // Fixing a duplicate must also shrink the allow-list, or the next one hides behind it.
        let mut by_value: HashMap<u32, usize> = HashMap::new();
        for (_, value) in declared_opcodes() {
            *by_value.entry(value).or_default() += 1;
        }

        let stale: Vec<String> = KNOWN_DUPLICATE_VALUES
            .iter()
            .filter(|value| by_value.get(value).copied().unwrap_or(0) < 2)
            .map(|value| format!("0x{value:04X}"))
            .collect();

        assert!(
            stale.is_empty(),
            "these values are no longer duplicated — remove them from KNOWN_DUPLICATE_VALUES: {}",
            stale.join(", ")
        );
    }

    #[test]
    fn the_all_table_lists_every_constant() {
        // ALL backs both the wire lookups and Debug, so a constant missing from it is invisible to
        // inbound resolution and prints as a bare number.
        let declared = declared_opcodes();
        assert_eq!(
            ALL.len(),
            declared.len(),
            "ALL has {} entries but {} constants are declared — regenerate it",
            ALL.len(),
            declared.len()
        );

        for (name, _) in &declared {
            assert!(
                ALL.iter().any(|(_, listed)| listed == name),
                "{name} is declared but missing from ALL"
            );
        }
    }

    #[test]
    fn vanilla_wire_numbers_round_trip() {
        // Skip the known duplicates: the index keeps the first declaration, so the second constant
        // sharing a value cannot round-trip until that data bug is fixed.
        for (opcode, name) in ALL {
            if !opcode.has_vanilla() || KNOWN_DUPLICATE_VALUES.contains(&opcode.vanilla()) {
                continue;
            }
            assert_eq!(
                Opcode::from_vanilla_wire(opcode.vanilla()),
                Some(*opcode),
                "{name} did not round-trip through the vanilla index"
            );
        }
    }

    /// Modern values shared by more than one constant.
    ///
    /// All three are the modern-side shadow of a vanilla duplicate already listed above — the
    /// generator resolved both spellings to the same modern opcode, which is correct. There is no
    /// modern-only collision, and there should never be one: the modern column is generated.
    const KNOWN_DUPLICATE_MODERN_VALUES: &[u16] = &[0x2701, 0x3265, 0x3A1A];

    #[test]
    fn no_unexpected_duplicate_modern_values() {
        let mut by_value: HashMap<u16, Vec<&str>> = HashMap::new();
        for (opcode, name) in ALL {
            if opcode.has_modern() {
                by_value.entry(opcode.modern()).or_default().push(name);
            }
        }

        let mut unexpected: Vec<String> = by_value
            .iter()
            .filter(|(value, names)| {
                names.len() > 1 && !KNOWN_DUPLICATE_MODERN_VALUES.contains(value)
            })
            .map(|(value, names)| format!("0x{value:04X}: {}", names.join(", ")))
            .collect();
        unexpected.sort();

        assert!(
            unexpected.is_empty(),
            "new duplicate modern opcode values:\n  {}",
            unexpected.join("\n  ")
        );
    }

    #[test]
    fn modern_wire_numbers_round_trip() {
        for (opcode, name) in ALL {
            if !opcode.has_modern() || KNOWN_DUPLICATE_MODERN_VALUES.contains(&opcode.modern()) {
                continue;
            }
            assert_eq!(
                Opcode::from_modern_wire(opcode.modern()),
                Some(*opcode),
                "{name} did not round-trip through the modern index"
            );
        }
    }

    /// Values observed on the wire from a live 1.14.2 client, plus the handshake opcodes that were
    /// hand-transcribed before this table existed. If the generator is ever re-run against a
    /// different build these are the first things that would silently change.
    #[test]
    fn modern_values_match_the_live_client() {
        assert_eq!(Opcode::CMSG_CHAR_ENUM.modern(), 0x35E9);
        assert_eq!(Opcode::CMSG_PING.modern(), 0x3768);
        assert_eq!(Opcode::SMSG_PONG.modern(), 0x304E);
        assert_eq!(Opcode::SMSG_AUTH_CHALLENGE.modern(), 0x3048);
        assert_eq!(Opcode::CMSG_AUTH_SESSION.modern(), 0x3765);
        assert_eq!(Opcode::SMSG_AUTH_RESPONSE.modern(), 0x256D);
        assert_eq!(Opcode::SMSG_ENTER_ENCRYPTED_MODE.modern(), 0x3049);
        assert_eq!(Opcode::CMSG_ENTER_ENCRYPTED_MODE_ACK.modern(), 0x3767);
        // The character list reply reuses the vanilla constant: the modern name for it is
        // SMSG_ENUM_CHARACTERS_RESULT, but it is the same message and the same handler.
        assert_eq!(Opcode::SMSG_CHAR_ENUM.modern(), 0x2583);
        assert_eq!(Opcode::CMSG_SERVER_TIME_OFFSET_REQUEST.modern(), 0x369B);
        assert_eq!(Opcode::CMSG_BATTLE_PAY_GET_PRODUCT_LIST.modern(), 0x36C2);
    }

    #[test]
    fn observed_modern_bootstrap_and_movement_opcodes_resolve() {
        for (wire, expected) in [
            (0x376C, Opcode::CMSG_QUEUED_MESSAGES_END),
            (0x3A45, Opcode::CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE),
            (0x39E4, Opcode::MSG_MOVE_START_FORWARD),
            (0x39E6, Opcode::MSG_MOVE_STOP),
            (0x39EA, Opcode::MSG_MOVE_JUMP),
            (0x3A07, Opcode::MSG_MOVE_SET_FACING),
            (0x3A08, Opcode::MSG_MOVE_SET_PITCH),
            (0x3A0F, Opcode::MSG_MOVE_HEARTBEAT),
            (0x3181, Opcode::CMSG_REQUEST_CATEGORY_COOLDOWNS),
            (0x3538, Opcode::CMSG_QUERY_NEXT_MAIL_TIME),
            (0x36BD, Opcode::CMSG_GET_ACCOUNT_CHARACTER_LIST),
        ] {
            assert_eq!(Opcode::from_modern_wire(wire), Some(expected));
        }
    }

    #[test]
    fn modern_only_opcodes_have_no_vanilla_number() {
        assert!(!Opcode::SMSG_CONNECT_TO.has_vanilla());
        assert!(Opcode::SMSG_CONNECT_TO.has_modern());
        assert_eq!(Opcode::from_vanilla_wire(0x304D), None);
    }

    #[test]
    fn unknown_wire_numbers_do_not_resolve() {
        assert_eq!(Opcode::from_vanilla_wire(0xFFFF), None);
        // Zero means "absent from this protocol" and must never resolve to a real opcode.
        assert_eq!(Opcode::from_vanilla_wire(0), None);
        assert_eq!(Opcode::from_modern_wire(0), None);
    }

    #[test]
    fn debug_prints_the_constant_name() {
        assert_eq!(format!("{:?}", Opcode::CMSG_PING), "CMSG_PING");
        assert_eq!(format!("{:?}", Opcode::CMSG_CHAR_ENUM), "CMSG_CHAR_ENUM");
    }

    #[test]
    fn debug_falls_back_to_numbers_for_an_unlisted_opcode() {
        // Constructed here rather than taken from ALL, which by definition contains no such value.
        let unlisted = Opcode {
            vanilla: 0xDEAD,
            modern: 0xBEEF,
        };
        assert_eq!(
            format!("{unlisted:?}"),
            "Opcode(vanilla=0xDEAD, modern=0xBEEF)"
        );
    }

    #[test]
    fn null_action_is_the_absent_opcode() {
        // `CMSG_NULL_ACTION` is 0 in both columns, so it is indistinguishable from `NONE`. Harmless
        // — 0 never resolves off the wire — but worth pinning so nobody reads meaning into it.
        assert_eq!(Opcode::CMSG_NULL_ACTION, Opcode::NONE);
        assert!(!Opcode::CMSG_NULL_ACTION.has_vanilla());
    }
}
