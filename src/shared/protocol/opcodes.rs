//! World server opcodes (Vanilla 1.12.x)
//!
//! Opcode is a wrapper around u16/u32 to represent packet opcodes.
//! This module is shared between world and world.

/// World server opcodes
/// Opcode is a wrapper around u16/u32 to represent packet opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opcode(u32);

impl Opcode {
    /// Create a new opcode from a u32 value
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Create from u16 (for server opcodes)
    pub fn from_u16(value: u16) -> Self {
        Self(value as u32)
    }

    /// Get as u16 (for server packets)
    pub fn as_u16(self) -> u16 {
        self.0 as u16
    }

    /// Get as u32 (for client packets)
    pub fn as_u32(self) -> u32 {
        self.0
    }

    // ============================================================================
    // Authentication & Connection
    // ============================================================================

    pub const CMSG_NULL_ACTION: Opcode = Opcode(0x000);
    pub const CMSG_PING: Opcode = Opcode(0x01DC); // 476
    pub const CMSG_AUTH_SESSION: Opcode = Opcode(0x01ED);
    pub const SMSG_AUTH_CHALLENGE: Opcode = Opcode(0x01EC);
    pub const SMSG_AUTH_RESPONSE: Opcode = Opcode(0x01EE);
    pub const SMSG_PONG: Opcode = Opcode(0x01D);

    // ============================================================================
    // Character Management
    // ============================================================================

    pub const CMSG_CHAR_CREATE: Opcode = Opcode(0x0036);
    pub const CMSG_CHAR_ENUM: Opcode = Opcode(0x0037);
    pub const CMSG_CHAR_DELETE: Opcode = Opcode(0x0038);
    pub const CMSG_PLAYER_LOGIN: Opcode = Opcode(0x003D);
    pub const CMSG_CHAR_RENAME: Opcode = Opcode(0x02C7); // 711
    pub const SMSG_CHAR_CREATE: Opcode = Opcode(0x003A);
    pub const SMSG_CHAR_ENUM: Opcode = Opcode(0x003B);
    pub const SMSG_CHAR_DELETE: Opcode = Opcode(0x003C);
    pub const SMSG_CHAR_RENAME: Opcode = Opcode(0x02C8); // 712
    pub const SMSG_CHARACTER_LOGIN_FAILED: Opcode = Opcode(0x0041); // 65

    // ============================================================================
    // Logout
    // ============================================================================

    pub const CMSG_LOGOUT_REQUEST: Opcode = Opcode(0x004B);
    pub const CMSG_LOGOUT_CANCEL: Opcode = Opcode(0x004E);
    pub const SMSG_LOGOUT_RESPONSE: Opcode = Opcode(0x004C);
    pub const SMSG_LOGOUT_COMPLETE: Opcode = Opcode(0x004D);
    pub const SMSG_LOGOUT_CANCEL_ACK: Opcode = Opcode(0x004F);

    // ============================================================================
    // World Entry & Time
    // ============================================================================

    pub const SMSG_NEW_WORLD: Opcode = Opcode(0x003E); // 62
    pub const SMSG_TRANSFER_PENDING: Opcode = Opcode(0x003F); // 63
    pub const SMSG_LOGIN_SETTIMESPEED: Opcode = Opcode(0x0042); // 66
    pub const SMSG_LOGIN_VERIFY_WORLD: Opcode = Opcode(0x0236); // 566
    pub const CMSG_QUERY_TIME: Opcode = Opcode(0x01CE); // 462
    pub const SMSG_QUERY_TIME_RESPONSE: Opcode = Opcode(0x01CF); // 463

    // ============================================================================
    // Tutorial & Account Data
    // ============================================================================

    pub const SMSG_TUTORIAL_FLAGS: Opcode = Opcode(0x00FD); // 253
    pub const CMSG_TUTORIAL_FLAG: Opcode = Opcode(0x00FE); // 254
    pub const CMSG_TUTORIAL_CLEAR: Opcode = Opcode(0x0100); // 256
    pub const CMSG_TUTORIAL_RESET: Opcode = Opcode(0x0101); // 257
    pub const CMSG_UPDATE_ACCOUNT_DATA: Opcode = Opcode(0x020B); // 523
    pub const SMSG_UPDATE_ACCOUNT_DATA: Opcode = Opcode(0x020C); // 524
    pub const CMSG_REQUEST_ACCOUNT_DATA: Opcode = Opcode(0x020A); // 522
    pub const SMSG_UPDATE_ACCOUNT_DATA_COMPLETE: Opcode = Opcode(0x020D); // 525
    pub const SMSG_ACCOUNT_DATA_MD5: Opcode = Opcode(0x0209); // 521
    pub const SMSG_ACCOUNT_DATA_TIMES: Opcode = Opcode(0x0209); // 521

    // ============================================================================
    // Query Responses
    // ============================================================================

    pub const CMSG_NAME_QUERY: Opcode = Opcode(0x050);
    pub const SMSG_NAME_QUERY_RESPONSE: Opcode = Opcode(0x051); // 81
    pub const CMSG_CREATURE_QUERY: Opcode = Opcode(0x060); // 96
    pub const SMSG_CREATURE_QUERY_RESPONSE: Opcode = Opcode(0x061); // 97
    pub const CMSG_ITEM_QUERY_SINGLE: Opcode = Opcode(0x056);
    pub const CMSG_ITEM_QUERY_MULTIPLE: Opcode = Opcode(0x057);
    pub const SMSG_ITEM_QUERY_SINGLE_RESPONSE: Opcode = Opcode(0x058); // 88
    pub const SMSG_ITEM_QUERY_MULTIPLE_RESPONSE: Opcode = Opcode(0x057);
    pub const CMSG_GAMEOBJECT_QUERY: Opcode = Opcode(0x005E); // 94
    pub const SMSG_GAMEOBJECT_QUERY_RESPONSE: Opcode = Opcode(0x005F); // 95
    pub const CMSG_PAGE_TEXT_QUERY: Opcode = Opcode(0x005A); // 90
    pub const SMSG_PAGE_TEXT_QUERY_RESPONSE: Opcode = Opcode(0x005B); // 91
    pub const CMSG_ITEM_TEXT_QUERY: Opcode = Opcode(0x0243); // 579
    pub const SMSG_ITEM_TEXT_QUERY_RESPONSE: Opcode = Opcode(0x0244); // 580
    pub const MSG_CORPSE_QUERY: Opcode = Opcode(0x0216); // 534
    pub const CMSG_ITEM_NAME_QUERY: Opcode = Opcode(0x02C4); // 708
    pub const SMSG_ITEM_NAME_QUERY_RESPONSE: Opcode = Opcode(0x02C5); // 709

    // ============================================================================
    // Object Updates
    // ============================================================================

    pub const SMSG_UPDATE_OBJECT: Opcode = Opcode(0x00A9); // 169
    pub const SMSG_COMPRESSED_UPDATE_OBJECT: Opcode = Opcode(0x01F6); // 502
    pub const SMSG_COMPRESSED_MOVES: Opcode = Opcode(0x02B3); // 691
    pub const SMSG_DESTROY_OBJECT: Opcode = Opcode(0x00AA); // 170

    // ============================================================================
    // Movement - Basic
    // ============================================================================

    pub const MSG_MOVE_HEARTBEAT: Opcode = Opcode(0x00EE); // 238
    pub const MSG_MOVE_START_FORWARD: Opcode = Opcode(0x00B5); // 181
    pub const MSG_MOVE_START_BACKWARD: Opcode = Opcode(0x00B6); // 182
    pub const MSG_MOVE_STOP: Opcode = Opcode(0x00B7); // 183
    pub const MSG_MOVE_START_STRAFE_LEFT: Opcode = Opcode(0x00B8); // 184
    pub const MSG_MOVE_START_STRAFE_RIGHT: Opcode = Opcode(0x00B9); // 185
    pub const MSG_MOVE_STOP_STRAFE: Opcode = Opcode(0x00BA); // 186
    pub const MSG_MOVE_JUMP: Opcode = Opcode(0x00BB); // 187
    pub const MSG_MOVE_START_TURN_LEFT: Opcode = Opcode(0x00BC); // 188
    pub const MSG_MOVE_START_TURN_RIGHT: Opcode = Opcode(0x00BD); // 189
    pub const MSG_MOVE_STOP_TURN: Opcode = Opcode(0x00BE); // 190
    pub const MSG_MOVE_SET_FACING: Opcode = Opcode(0x00DA); // 218
    pub const MSG_MOVE_SET_PITCH: Opcode = Opcode(0x00DB); // 219
    pub const MSG_MOVE_WORLDPORT_ACK: Opcode = Opcode(0x00DC); // 220
    pub const MSG_MOVE_FALL_LAND: Opcode = Opcode(0x00C9); // 201

    // ============================================================================
    // Movement - Advanced
    // ============================================================================

    pub const CMSG_SET_ACTIVE_MOVER: Opcode = Opcode(0x026A); // 618
    pub const CMSG_MOVE_SPLINE_DONE: Opcode = Opcode(0x02C9); // 713
    pub const CMSG_MOVE_FALL_RESET: Opcode = Opcode(0x02CA); // 714
    pub const CMSG_MOVE_TIME_SKIPPED: Opcode = Opcode(0x02CE); // 718
    pub const CMSG_MOVE_FEATHER_FALL_ACK: Opcode = Opcode(0x02CF); // 719
    pub const CMSG_MOVE_WATER_WALK_ACK: Opcode = Opcode(0x02D0); // 720
    pub const CMSG_MOVE_NOT_ACTIVE_MOVER: Opcode = Opcode(0x02D1); // 721
    pub const MSG_MOVE_TELEPORT_ACK: Opcode = Opcode(0x00C7); // 199
    pub const MSG_MOVE_TELEPORT: Opcode = Opcode(0x00C5); // 197
    pub const MSG_MOVE_KNOCK_BACK: Opcode = Opcode(0x00F1); // 241

    // ============================================================================
    // Movement - Speed Changes (Force - to controller)
    // ============================================================================

    pub const SMSG_FORCE_WALK_SPEED_CHANGE: Opcode = Opcode(0x02DA); // 730
    pub const SMSG_FORCE_RUN_SPEED_CHANGE: Opcode = Opcode(0x00E2); // 226
    pub const SMSG_FORCE_RUN_BACK_SPEED_CHANGE: Opcode = Opcode(0x00E4); // 228
    pub const SMSG_FORCE_SWIM_SPEED_CHANGE: Opcode = Opcode(0x00E6); // 230
    pub const SMSG_FORCE_SWIM_BACK_SPEED_CHANGE: Opcode = Opcode(0x02DC); // 732
    pub const SMSG_FORCE_TURN_RATE_CHANGE: Opcode = Opcode(0x02DE); // 734

    // ============================================================================
    // Movement - Speed Changes (Spline - server-controlled units)
    // ============================================================================

    pub const SMSG_SPLINE_SET_WALK_SPEED: Opcode = Opcode(0x0301); // 769
    pub const SMSG_SPLINE_SET_RUN_SPEED: Opcode = Opcode(0x02FE); // 766
    pub const SMSG_SPLINE_SET_RUN_BACK_SPEED: Opcode = Opcode(0x02FF); // 767
    pub const SMSG_SPLINE_SET_SWIM_SPEED: Opcode = Opcode(0x0300); // 768
    pub const SMSG_SPLINE_SET_SWIM_BACK_SPEED: Opcode = Opcode(0x0302); // 770
    pub const SMSG_SPLINE_SET_TURN_RATE: Opcode = Opcode(0x0303); // 771

    // ============================================================================
    // Movement - Speed Changes (MSG - to observers)
    // ============================================================================

    pub const MSG_MOVE_SET_WALK_SPEED: Opcode = Opcode(0x00D1); // 209
    pub const MSG_MOVE_SET_RUN_SPEED: Opcode = Opcode(0x00CD); // 205
    pub const MSG_MOVE_SET_RUN_BACK_SPEED: Opcode = Opcode(0x00CF); // 207
    pub const MSG_MOVE_SET_SWIM_SPEED: Opcode = Opcode(0x00D3); // 211
    pub const MSG_MOVE_SET_SWIM_BACK_SPEED: Opcode = Opcode(0x00D5); // 213
    pub const MSG_MOVE_SET_TURN_RATE: Opcode = Opcode(0x00D8); // 216

    // ============================================================================
    // Movement - Flags (Force - to controller)
    // ============================================================================

    pub const SMSG_FORCE_MOVE_ROOT: Opcode = Opcode(0x00E8); // 232
    pub const CMSG_FORCE_MOVE_ROOT_ACK: Opcode = Opcode(0x00E9); // 233
    pub const SMSG_FORCE_MOVE_UNROOT: Opcode = Opcode(0x00EA); // 234
    pub const SMSG_MOVE_WATER_WALK: Opcode = Opcode(0x00DE); // 222
    pub const SMSG_MOVE_LAND_WALK: Opcode = Opcode(0x00DF); // 223
    pub const SMSG_MOVE_SET_HOVER: Opcode = Opcode(0x00F4); // 244
    pub const SMSG_MOVE_UNSET_HOVER: Opcode = Opcode(0x00F5); // 245
    pub const SMSG_MOVE_FEATHER_FALL: Opcode = Opcode(0x00F2); // 242
    pub const SMSG_MOVE_NORMAL_FALL: Opcode = Opcode(0x00F3); // 243
    pub const SMSG_MOVE_KNOCK_BACK: Opcode = Opcode(0x00EF); // 239

    // ============================================================================
    // Movement - Flags (Spline - server-controlled units)
    // ============================================================================

    pub const SMSG_SPLINE_MOVE_ROOT: Opcode = Opcode(0x031A); // 794
    pub const SMSG_SPLINE_MOVE_UNROOT: Opcode = Opcode(0x0304); // 772
    pub const SMSG_SPLINE_MOVE_WATER_WALK: Opcode = Opcode(0x0309); // 777
    pub const SMSG_SPLINE_MOVE_LAND_WALK: Opcode = Opcode(0x030A); // 778
    pub const SMSG_SPLINE_MOVE_SET_HOVER: Opcode = Opcode(0x0307); // 775
    pub const SMSG_SPLINE_MOVE_UNSET_HOVER: Opcode = Opcode(0x0308); // 776
    pub const SMSG_SPLINE_MOVE_FEATHER_FALL: Opcode = Opcode(0x0305); // 773
    pub const SMSG_SPLINE_MOVE_NORMAL_FALL: Opcode = Opcode(0x0306); // 774
    pub const SMSG_SPLINE_MOVE_SET_RUN_MODE: Opcode = Opcode(0x030D); // 781
    pub const SMSG_SPLINE_MOVE_SET_WALK_MODE: Opcode = Opcode(0x030E); // 782

    // ============================================================================
    // Movement - Flags (MSG - to observers)
    // ============================================================================

    pub const MSG_MOVE_ROOT: Opcode = Opcode(0x00EC); // 236
    pub const MSG_MOVE_UNROOT: Opcode = Opcode(0x00ED); // 237
    pub const MSG_MOVE_WATER_WALK: Opcode = Opcode(0x02B1); // 689
    pub const MSG_MOVE_HOVER: Opcode = Opcode(0x00F7); // 247
    pub const MSG_MOVE_FEATHER_FALL: Opcode = Opcode(0x02B0); // 688

    // ============================================================================
    // Monster Movement
    // ============================================================================

    pub const SMSG_MONSTER_MOVE: Opcode = Opcode(0x00DD); // 221
    pub const SMSG_MONSTER_MOVE_TRANSPORT: Opcode = Opcode(0x02AE); // 686

    // ============================================================================
    // Combat
    // ============================================================================

    pub const CMSG_ATTACKSWING: Opcode = Opcode(0x0141); // 321
    pub const CMSG_ATTACKSTOP: Opcode = Opcode(0x0142); // 322
    pub const SMSG_ATTACKSTART: Opcode = Opcode(0x0143); // 323
    pub const SMSG_ATTACKSTOP: Opcode = Opcode(0x0144); // 324
    pub const SMSG_ATTACKSWING_NOTINRANGE: Opcode = Opcode(0x0145); // 325
    pub const SMSG_ATTACKSWING_BADFACING: Opcode = Opcode(0x0146); // 326
    pub const SMSG_ATTACKSWING_NOTSTANDING: Opcode = Opcode(0x0147); // 327
    pub const SMSG_ATTACKSWING_DEADTARGET: Opcode = Opcode(0x0148); // 328
    pub const SMSG_ATTACKSWING_CANT_ATTACK: Opcode = Opcode(0x0149); // 329
    pub const SMSG_ATTACKERSTATEUPDATE: Opcode = Opcode(0x014A); // 330

    // ============================================================================
    // Selection & Targeting
    // ============================================================================

    pub const CMSG_SET_SELECTION: Opcode = Opcode(0x013D); // 317

    // ============================================================================
    // Stand State
    // ============================================================================

    pub const CMSG_STANDSTATECHANGE: Opcode = Opcode(0x0101); // 257
    pub const SMSG_STANDSTATE_UPDATE: Opcode = Opcode(0x029D); // 669

    // ============================================================================
    // Spell Casting
    // ============================================================================

    pub const CMSG_CAST_SPELL: Opcode = Opcode(0x012E); // 302
    pub const CMSG_CANCEL_CAST: Opcode = Opcode(0x012F); // 303
    pub const CMSG_CANCEL_AURA: Opcode = Opcode(0x0136); // 310
    pub const CMSG_CANCEL_AUTO_REPEAT_SPELL: Opcode = Opcode(0x026D); // 621
    pub const CMSG_CANCEL_CHANNELING: Opcode = Opcode(0x013B); // 315
    pub const CMSG_CANCEL_CHANNELLING: Opcode = Opcode(0x013B); // 315 (alias)
    pub const CMSG_USE_ITEM: Opcode = Opcode(0x00AB); // 171
    pub const CMSG_NEW_SPELL_SLOT: Opcode = Opcode(0x012D); // 301
    pub const SMSG_SPELL_START: Opcode = Opcode(0x0131); // 305
    pub const SMSG_SPELL_GO: Opcode = Opcode(0x0132); // 306
    pub const SMSG_CAST_RESULT: Opcode = Opcode(0x0130); // 304
    pub const SMSG_SPELL_COOLDOWN: Opcode = Opcode(0x0134); // 308
    pub const MSG_CHANNEL_START: Opcode = Opcode(0x0139); // 313
    pub const MSG_CHANNEL_UPDATE: Opcode = Opcode(0x013A); // 314
    pub const SMSG_SPELL_INTERRUPTED: Opcode = Opcode(0x0152); // 338
    pub const SMSG_SPELL_DELAYED: Opcode = Opcode(0x01E2); // 482
    pub const SMSG_SPELL_FAILED_OTHER: Opcode = Opcode(0x02A6); // 678
    pub const SMSG_SPELL_UPDATE_CHAIN_TARGETS: Opcode = Opcode(0x0330); // 816
    pub const SMSG_SET_PROFICIENCY: Opcode = Opcode(0x0127); // 295
    pub const SMSG_INITIAL_SPELLS: Opcode = Opcode(0x012A); // 298
    pub const SMSG_LEARNED_SPELL: Opcode = Opcode(0x012B); // 299
    pub const SMSG_REMOVED_SPELL: Opcode = Opcode(0x0203); // 515
    pub const SMSG_SPELL_FAILURE: Opcode = Opcode(0x0133); // 307
    pub const SMSG_CLEAR_COOLDOWN: Opcode = Opcode(0x01DE); // 478

    // ============================================================================
    // Auras
    // ============================================================================

    pub const SMSG_AURA_UPDATE: Opcode = Opcode(0x0495); // 1173
    pub const SMSG_AURA_UPDATE_ALL: Opcode = Opcode(0x0496); // 1174
    pub const SMSG_UPDATE_AURA_DURATION: Opcode = Opcode(0x0137); // 311
    pub const SMSG_SET_EXTRA_AURA_INFO: Opcode = Opcode(0x04A7); // 1191
    pub const SMSG_PERIODICAURALOG: Opcode = Opcode(0x024E); // 590

    // ============================================================================
    // Combat Log
    // ============================================================================

    pub const SMSG_SPELLDAMAGELOG: Opcode = Opcode(0x014E); // 334
    pub const SMSG_SPELLHEALLOG: Opcode = Opcode(0x0150); // 336
    pub const SMSG_SPELLLOGMISS: Opcode = Opcode(0x014C); // 332
    pub const SMSG_SPELLENERGIZELOG: Opcode = Opcode(0x0151); // 337
    pub const SMSG_SPELLNONMELEEDAMAGELOG: Opcode = Opcode(0x0148); // 328
    pub const SMSG_SPELLLOGEXECUTE: Opcode = Opcode(0x024C); // 588
    pub const SMSG_SPELLINSTAKILLLOG: Opcode = Opcode(0x033F); // 815

    // ============================================================================
    // Action Bar
    // ============================================================================

    pub const CMSG_SET_ACTION_BUTTON: Opcode = Opcode(0x0128); // 296
    pub const SMSG_ACTION_BUTTONS: Opcode = Opcode(0x0129); // 297

    // ============================================================================
    // Death & Resurrection
    // ============================================================================

    pub const SMSG_DURABILITY_DAMAGE_DEATH: Opcode = Opcode(0x02BD); // 701
    pub const SMSG_CORPSE_RECLAIM_DELAY: Opcode = Opcode(0x0269); // 617
    pub const CMSG_REPOP_REQUEST: Opcode = Opcode(0x015A); // 346
    pub const CMSG_RESURRECT_RESPONSE: Opcode = Opcode(0x015C); // 348
    pub const CMSG_RECLAIM_CORPSE: Opcode = Opcode(0x01D2); // 466
    pub const SMSG_RESURRECT_REQUEST: Opcode = Opcode(0x015B); // 347
    pub const SMSG_SPIRIT_HEALER_CONFIRM: Opcode = Opcode(0x0222); // 546
    pub const CMSG_SPIRIT_HEALER_ACTIVATE: Opcode = Opcode(0x021C); // 540
    pub const CMSG_SELF_RES: Opcode = Opcode(0x02B3); // 691

    // ============================================================================
    // NPC Interaction - Gossip
    // ============================================================================

    pub const CMSG_GOSSIP_HELLO: Opcode = Opcode(0x017B); // 379
    pub const CMSG_GOSSIP_SELECT_OPTION: Opcode = Opcode(0x017C); // 380
    pub const SMSG_GOSSIP_MESSAGE: Opcode = Opcode(0x017D); // 381
    pub const SMSG_GOSSIP_COMPLETE: Opcode = Opcode(0x017E); // 382
    pub const SMSG_GOSSIP_POI: Opcode = Opcode(0x0223); // 547
    pub const SMSG_NPC_TEXT_UPDATE: Opcode = Opcode(0x0180); // 384
    pub const CMSG_NPC_TEXT_QUERY: Opcode = Opcode(0x017F); // 383

    // ============================================================================
    // NPC Interaction - Vendor
    // ============================================================================

    pub const CMSG_LIST_INVENTORY: Opcode = Opcode(0x019E); // 414
    pub const SMSG_LIST_INVENTORY: Opcode = Opcode(0x019F); // 415
    pub const CMSG_SELL_ITEM: Opcode = Opcode(0x01A0); // 416
    pub const SMSG_SELL_ITEM: Opcode = Opcode(0x01A1); // 417
    pub const CMSG_BUY_ITEM: Opcode = Opcode(0x01A2); // 418
    pub const CMSG_BUY_ITEM_IN_SLOT: Opcode = Opcode(0x01A3); // 419
    pub const SMSG_BUY_ITEM: Opcode = Opcode(0x01A4); // 420
    pub const SMSG_BUY_FAILED: Opcode = Opcode(0x01A5); // 421
    pub const SMSG_ITEM_PUSH_RESULT: Opcode = Opcode(0x0166); // 358
    pub const CMSG_BUYBACK_ITEM: Opcode = Opcode(0x0290); // 656

    // ============================================================================
    // NPC Interaction - Trainer
    // ============================================================================

    pub const CMSG_TRAINER_LIST: Opcode = Opcode(0x01B0); // 432
    pub const SMSG_TRAINER_LIST: Opcode = Opcode(0x01B1); // 433
    pub const CMSG_TRAINER_BUY_SPELL: Opcode = Opcode(0x01B2); // 434
    pub const SMSG_TRAINER_BUY_SUCCEEDED: Opcode = Opcode(0x01B3); // 435
    pub const SMSG_TRAINER_BUY_FAILED: Opcode = Opcode(0x01B4); // 436

    // ============================================================================
    // NPC Interaction - Banker
    // ============================================================================

    pub const CMSG_BANKER_ACTIVATE: Opcode = Opcode(0x01B5); // 439
    pub const SMSG_SHOW_BANK: Opcode = Opcode(0x01B8); // 440
    pub const CMSG_BUY_BANK_SLOT: Opcode = Opcode(0x01B9); // 441
    pub const SMSG_BUY_BANK_SLOT_RESULT: Opcode = Opcode(0x0216); // 534
    pub const CMSG_AUTOBANK_ITEM: Opcode = Opcode(0x0283); // 643
    pub const CMSG_AUTOSTORE_BANK_ITEM: Opcode = Opcode(0x0282); // 642

    // ============================================================================
    // NPC Interaction - Other
    // ============================================================================

    pub const CMSG_BINDER_ACTIVATE: Opcode = Opcode(0x01B5); // 437
    pub const MSG_TABARDVENDOR_ACTIVATE: Opcode = Opcode(0x01F2); // 498

    // ============================================================================
    // Taxi
    // ============================================================================

    pub const CMSG_TAXINODE_STATUS_QUERY: Opcode = Opcode(0x01AA); // 426
    pub const SMSG_TAXINODE_STATUS: Opcode = Opcode(0x01AB); // 427
    pub const CMSG_TAXIQUERYAVAILABLENODES: Opcode = Opcode(0x01AC); // 428
    pub const SMSG_SHOWTAXINODES: Opcode = Opcode(0x01A9); // 425
    pub const CMSG_ACTIVATETAXI: Opcode = Opcode(0x01AD); // 429
    pub const SMSG_ACTIVATETAXIREPLY: Opcode = Opcode(0x01AE); // 430
    pub const SMSG_NEW_TAXI_PATH: Opcode = Opcode(0x01AF); // 431

    // ============================================================================
    // Talents & Skills
    // ============================================================================

    pub const CMSG_LEARN_TALENT: Opcode = Opcode(0x0251); // 593
    pub const CMSG_UNLEARN_TALENTS: Opcode = Opcode(0x0213); // 531
    pub const CMSG_UNLEARN_SPELL: Opcode = Opcode(0x0201); // 513
    pub const CMSG_UNLEARN_SKILL: Opcode = Opcode(0x0202); // 514

    // ============================================================================
    // Bind Point
    // ============================================================================

    pub const SMSG_BINDPOINTUPDATE: Opcode = Opcode(0x0155); // 341
    pub const SMSG_BINDZONEREPLY: Opcode = Opcode(0x0157); // 343
    pub const SMSG_PLAYERBOUND: Opcode = Opcode(0x0158); // 344
    pub const CMSG_SETDEATHBINDPOINT: Opcode = Opcode(0x0154); // 340
    pub const CMSG_GETDEATHBINDZONE: Opcode = Opcode(0x0156); // 342

    // ============================================================================
    // Rest & XP
    // ============================================================================

    pub const SMSG_SET_REST_START: Opcode = Opcode(0x021E); // 542
    pub const SMSG_LOG_XPGAIN: Opcode = Opcode(0x01D0); // 464
    pub const SMSG_LEVELUP_INFO: Opcode = Opcode(0x01D4); // 468

    // ============================================================================
    // Environment & Mirror Timers
    // ============================================================================

    pub const SMSG_START_MIRROR_TIMER: Opcode = Opcode(0x0C1D); // 3101
    pub const SMSG_STOP_MIRROR_TIMER: Opcode = Opcode(0x0C1E); // 3102
    pub const SMSG_ENVIRONMENTALDAMAGELOG: Opcode = Opcode(0x0C1F); // 3103
    pub const SMSG_EXPLORATION_EXPERIENCE: Opcode = Opcode(0x01F8); // 504

    // ============================================================================
    // World States & Factions
    // ============================================================================

    pub const SMSG_INIT_WORLD_STATES: Opcode = Opcode(0x02C2); // 706
    pub const SMSG_INITIALIZE_FACTIONS: Opcode = Opcode(0x0122); // 290
    pub const SMSG_SET_FACTION_STANDING: Opcode = Opcode(0x0124); // 292
    pub const SMSG_SET_FACTION_VISIBLE: Opcode = Opcode(0x0123); // 291
    pub const SMSG_SET_FORCED_REACTIONS: Opcode = Opcode(0x02A5); // 677
    pub const CMSG_SET_FACTION_ATWAR: Opcode = Opcode(0x0125); // 293
    pub const CMSG_SET_FACTION_INACTIVE: Opcode = Opcode(0x0317); // 791

    // ============================================================================
    // Cinematic
    // ============================================================================

    pub const SMSG_TRIGGER_CINEMATIC: Opcode = Opcode(0x00FA); // 250
    pub const CMSG_NEXT_CINEMATIC_CAMERA: Opcode = Opcode(0x00FB); // 251
    pub const CMSG_COMPLETE_CINEMATIC: Opcode = Opcode(0x00FC); // 252

    // ============================================================================
    // Zone
    // ============================================================================

    pub const CMSG_ZONEUPDATE: Opcode = Opcode(0x01F4); // 500

    // ============================================================================
    // Item Management
    // ============================================================================

    pub const CMSG_OPEN_ITEM: Opcode = Opcode(0x00AC); // 172
    pub const CMSG_READ_ITEM: Opcode = Opcode(0x00AD); // 173
    pub const SMSG_READ_ITEM_OK: Opcode = Opcode(0x00AE); // 174
    pub const SMSG_READ_ITEM_FAILED: Opcode = Opcode(0x00AF); // 175
    pub const SMSG_ITEM_COOLDOWN: Opcode = Opcode(0x00B0); // 176
    pub const SMSG_INVENTORY_CHANGE_FAILURE: Opcode = Opcode(0x0112); // 274
    pub const CMSG_AUTOEQUIP_GROUND_ITEM: Opcode = Opcode(0x0106); // 262
    pub const CMSG_AUTOSTORE_GROUND_ITEM: Opcode = Opcode(0x0107); // 263
    pub const CMSG_AUTOSTORE_LOOT_ITEM: Opcode = Opcode(0x0108); // 264
    pub const CMSG_STORE_LOOT_IN_SLOT: Opcode = Opcode(0x0109); // 265
    pub const CMSG_AUTOEQUIP_ITEM: Opcode = Opcode(0x010A); // 266
    pub const CMSG_AUTOSTORE_BAG_ITEM: Opcode = Opcode(0x010B); // 267
    pub const CMSG_SWAP_ITEM: Opcode = Opcode(0x010C); // 268
    pub const CMSG_SWAP_INV_ITEM: Opcode = Opcode(0x010D); // 269
    pub const CMSG_SPLIT_ITEM: Opcode = Opcode(0x010E); // 270
    pub const CMSG_AUTOEQUIP_ITEM_SLOT: Opcode = Opcode(0x010F); // 271
    pub const CMSG_DROP_ITEM: Opcode = Opcode(0x0110); // 272
    pub const CMSG_DESTROYITEM: Opcode = Opcode(0x0111); // 273
    pub const CMSG_INSPECT: Opcode = Opcode(0x0114); // 276
    pub const SMSG_INSPECT: Opcode = Opcode(0x0115); // 277
    pub const MSG_INSPECT_HONOR_STATS: Opcode = Opcode(0x02D6); // 726
    pub const CMSG_REPAIR_ITEM: Opcode = Opcode(0x02A8); // 680
    pub const SMSG_ITEM_TIME_UPDATE: Opcode = Opcode(0x01EB); // 491
    pub const SMSG_ITEM_ENCHANT_TIME_UPDATE: Opcode = Opcode(0x01EC); // 492
    pub const CMSG_SET_AMMO: Opcode = Opcode(0x0268); // 619
    pub const CMSG_WRAP_ITEM: Opcode = Opcode(0x01D3); // 467

    // ============================================================================
    // Gameobject
    // ============================================================================

    pub const CMSG_GAMEOBJ_USE: Opcode = Opcode(0x00B1); // 177

    // ============================================================================
    // Area Trigger
    // ============================================================================

    pub const CMSG_AREATRIGGER: Opcode = Opcode(0x00B4); // 180

    // ============================================================================
    // Chat
    // ============================================================================

    pub const CMSG_MESSAGECHAT: Opcode = Opcode(0x0095); // 149
    pub const SMSG_MESSAGECHAT: Opcode = Opcode(0x0096); // 150
    pub const CMSG_CHAT_IGNORED: Opcode = Opcode(0x0225); // 549
    pub const SMSG_CHAT_WRONG_FACTION: Opcode = Opcode(0x0219); // 537
    pub const SMSG_CHAT_PLAYER_NOT_FOUND: Opcode = Opcode(0x02A9); // 681
    pub const SMSG_CHAT_RESTRICTED: Opcode = Opcode(0x02FD); // 765
    pub const SMSG_CHAT_PLAYER_AMBIGUOUS: Opcode = Opcode(0x032D); // 813
    pub const CMSG_CHAT_FILTERED: Opcode = Opcode(0x0331); // 817

    // ============================================================================
    // Emote
    // ============================================================================

    pub const CMSG_EMOTE: Opcode = Opcode(0x0102); // 258
    pub const CMSG_TEXT_EMOTE: Opcode = Opcode(0x0104); // 260
    pub const SMSG_TEXT_EMOTE: Opcode = Opcode(0x0105); // 261
    pub const SMSG_EMOTE: Opcode = Opcode(0x0103); // 259
    pub const SMSG_PLAY_OBJECT_SOUND: Opcode = Opcode(0x0278); // 632
    pub const SMSG_PLAY_SOUND: Opcode = Opcode(0x02D2); // 722
    pub const SMSG_PLAY_SPELL_VISUAL: Opcode = Opcode(0x01F3); // 499

    // ============================================================================
    // Channel
    // ============================================================================

    pub const CMSG_JOIN_CHANNEL: Opcode = Opcode(0x0097); // 151
    pub const CMSG_LEAVE_CHANNEL: Opcode = Opcode(0x0098); // 152
    pub const SMSG_CHANNEL_NOTIFY: Opcode = Opcode(0x0099); // 153
    pub const CMSG_CHANNEL_LIST: Opcode = Opcode(0x009A); // 154
    pub const SMSG_CHANNEL_LIST: Opcode = Opcode(0x009B); // 155
    pub const CMSG_CHANNEL_PASSWORD: Opcode = Opcode(0x009C); // 156
    pub const CMSG_CHANNEL_SET_OWNER: Opcode = Opcode(0x009D); // 157
    pub const CMSG_CHANNEL_OWNER: Opcode = Opcode(0x009E); // 158
    pub const CMSG_CHANNEL_MODERATOR: Opcode = Opcode(0x009F); // 159
    pub const CMSG_CHANNEL_UNMODERATOR: Opcode = Opcode(0x00A0); // 160
    pub const CMSG_CHANNEL_MUTE: Opcode = Opcode(0x00A1); // 161
    pub const CMSG_CHANNEL_UNMUTE: Opcode = Opcode(0x00A2); // 162
    pub const CMSG_CHANNEL_INVITE: Opcode = Opcode(0x00A3); // 163
    pub const CMSG_CHANNEL_KICK: Opcode = Opcode(0x00A4); // 164
    pub const CMSG_CHANNEL_BAN: Opcode = Opcode(0x00A5); // 165
    pub const CMSG_CHANNEL_UNBAN: Opcode = Opcode(0x00A6); // 166
    pub const CMSG_CHANNEL_ANNOUNCEMENTS: Opcode = Opcode(0x00A7); // 167
    pub const CMSG_CHANNEL_MODERATE: Opcode = Opcode(0x00A8); // 168

    // ============================================================================
    // Social - Who & Friends
    // ============================================================================

    pub const CMSG_WHO: Opcode = Opcode(0x0062); // 98
    pub const SMSG_WHO: Opcode = Opcode(0x0063); // 99
    pub const CMSG_FRIEND_LIST: Opcode = Opcode(0x0066); // 102
    pub const SMSG_FRIEND_LIST: Opcode = Opcode(0x0067); // 103
    pub const SMSG_FRIEND_STATUS: Opcode = Opcode(0x0068); // 104
    pub const CMSG_ADD_FRIEND: Opcode = Opcode(0x0069); // 105
    pub const CMSG_DEL_FRIEND: Opcode = Opcode(0x006A); // 106
    pub const SMSG_IGNORE_LIST: Opcode = Opcode(0x006B); // 107
    pub const CMSG_ADD_IGNORE: Opcode = Opcode(0x006C); // 108
    pub const CMSG_DEL_IGNORE: Opcode = Opcode(0x006D); // 109

    // ============================================================================
    // Group
    // ============================================================================

    pub const CMSG_GROUP_INVITE: Opcode = Opcode(0x006E); // 110
    pub const SMSG_GROUP_INVITE: Opcode = Opcode(0x006F); // 111
    pub const MSG_PARTY_LEAVE: Opcode = Opcode(0x0071); // 113
    pub const CMSG_GROUP_ACCEPT: Opcode = Opcode(0x0072); // 114
    pub const CMSG_GROUP_DECLINE: Opcode = Opcode(0x0073); // 115
    pub const SMSG_GROUP_DECLINE: Opcode = Opcode(0x0074); // 116
    pub const CMSG_GROUP_UNINVITE: Opcode = Opcode(0x0075); // 117
    pub const SMSG_GROUP_UNINVITE: Opcode = Opcode(0x0077); // 119
    pub const CMSG_GROUP_SET_LEADER: Opcode = Opcode(0x0078); // 120
    pub const SMSG_GROUP_SET_LEADER: Opcode = Opcode(0x0079); // 121
    pub const CMSG_LOOT_METHOD: Opcode = Opcode(0x007A); // 122
    pub const CMSG_GROUP_DISBAND: Opcode = Opcode(0x007B); // 123
    pub const SMSG_GROUP_DESTROYED: Opcode = Opcode(0x007C); // 124
    pub const SMSG_GROUP_LIST: Opcode = Opcode(0x007D); // 125
    pub const SMSG_PARTY_MEMBER_STATS: Opcode = Opcode(0x007E); // 126
    pub const SMSG_PARTY_COMMAND_RESULT: Opcode = Opcode(0x007F); // 127
    pub const CMSG_GROUP_CHANGE_SUB_GROUP: Opcode = Opcode(0x027E); // 638
    pub const CMSG_GROUP_SWAP_SUB_GROUP: Opcode = Opcode(0x0280); // 640
    pub const CMSG_GROUP_ASSISTANT_LEADER: Opcode = Opcode(0x028F); // 655
    pub const CMSG_GROUP_RAID_CONVERT: Opcode = Opcode(0x028E); // 654
    pub const CMSG_REQUEST_PARTY_MEMBER_STATS: Opcode = Opcode(0x027F); // 639
    pub const SMSG_PARTY_MEMBER_STATS_FULL: Opcode = Opcode(0x02F2); // 754
    pub const MSG_RAID_TARGET_UPDATE: Opcode = Opcode(0x0321); // 801
    pub const MSG_RAID_READY_CHECK: Opcode = Opcode(0x0322); // 802
    pub const MSG_MINIMAP_PING: Opcode = Opcode(0x01D5); // 469
    pub const MSG_RANDOM_ROLL: Opcode = Opcode(0x01FB); // 507

    // ============================================================================
    // Loot
    // ============================================================================

    pub const CMSG_LOOT: Opcode = Opcode(0x015D); // 349
    pub const CMSG_LOOT_MONEY: Opcode = Opcode(0x015E); // 350
    pub const CMSG_LOOT_RELEASE: Opcode = Opcode(0x015F); // 351
    pub const SMSG_LOOT_RESPONSE: Opcode = Opcode(0x0160); // 352
    pub const SMSG_LOOT_RELEASE_RESPONSE: Opcode = Opcode(0x0161); // 353
    pub const SMSG_LOOT_REMOVED: Opcode = Opcode(0x0162); // 354
    pub const SMSG_LOOT_MONEY_NOTIFY: Opcode = Opcode(0x0163); // 355
    pub const SMSG_LOOT_CLEAR_MONEY: Opcode = Opcode(0x0165); // 357
    pub const CMSG_LOOT_ROLL: Opcode = Opcode(0x02A0); // 672
    pub const SMSG_LOOT_START_ROLL: Opcode = Opcode(0x02A1); // 673
    pub const SMSG_LOOT_ROLL: Opcode = Opcode(0x02A2); // 674
    pub const CMSG_LOOT_MASTER_GIVE: Opcode = Opcode(0x02A3); // 675
    pub const SMSG_LOOT_MASTER_LIST: Opcode = Opcode(0x02A4); // 676
    pub const SMSG_LOOT_ROLL_WON: Opcode = Opcode(0x029F); // 671
    pub const SMSG_LOOT_ALL_PASSED: Opcode = Opcode(0x029E); // 670

    // ============================================================================
    // Trade
    // ============================================================================

    pub const CMSG_INITIATE_TRADE: Opcode = Opcode(0x0116); // 278
    pub const CMSG_BEGIN_TRADE: Opcode = Opcode(0x0117); // 279
    pub const CMSG_BUSY_TRADE: Opcode = Opcode(0x0118); // 280
    pub const CMSG_IGNORE_TRADE: Opcode = Opcode(0x0119); // 281
    pub const CMSG_ACCEPT_TRADE: Opcode = Opcode(0x011A); // 282
    pub const CMSG_UNACCEPT_TRADE: Opcode = Opcode(0x011B); // 283
    pub const CMSG_CANCEL_TRADE: Opcode = Opcode(0x011C); // 284
    pub const CMSG_SET_TRADE_ITEM: Opcode = Opcode(0x011D); // 285
    pub const CMSG_CLEAR_TRADE_ITEM: Opcode = Opcode(0x011E); // 286
    pub const CMSG_SET_TRADE_GOLD: Opcode = Opcode(0x011F); // 287
    pub const SMSG_TRADE_STATUS: Opcode = Opcode(0x0120); // 288
    pub const SMSG_TRADE_STATUS_EXTENDED: Opcode = Opcode(0x0121); // 289

    // ============================================================================
    // Quest
    // ============================================================================

    pub const CMSG_QUEST_QUERY: Opcode = Opcode(0x005C); // 92
    pub const SMSG_QUEST_QUERY_RESPONSE: Opcode = Opcode(0x005D); // 93
    pub const CMSG_QUESTGIVER_STATUS_QUERY: Opcode = Opcode(0x0182); // 386
    pub const SMSG_QUESTGIVER_STATUS: Opcode = Opcode(0x0183); // 387
    pub const CMSG_QUESTGIVER_HELLO: Opcode = Opcode(0x0184); // 388
    pub const SMSG_QUESTGIVER_QUEST_LIST: Opcode = Opcode(0x0185); // 389
    pub const CMSG_QUESTGIVER_QUERY_QUEST: Opcode = Opcode(0x0186); // 390
    pub const CMSG_QUESTGIVER_QUEST_AUTOLAUNCH: Opcode = Opcode(0x0187); // 391
    pub const SMSG_QUESTGIVER_QUEST_DETAILS: Opcode = Opcode(0x0188); // 392
    pub const CMSG_QUESTGIVER_ACCEPT_QUEST: Opcode = Opcode(0x0189); // 393
    pub const CMSG_QUESTGIVER_COMPLETE_QUEST: Opcode = Opcode(0x018A); // 394
    pub const SMSG_QUESTGIVER_REQUEST_ITEMS: Opcode = Opcode(0x018B); // 395
    pub const CMSG_QUESTGIVER_REQUEST_REWARD: Opcode = Opcode(0x018C); // 396
    pub const SMSG_QUESTGIVER_OFFER_REWARD: Opcode = Opcode(0x018D); // 397
    pub const CMSG_QUESTGIVER_CHOOSE_REWARD: Opcode = Opcode(0x018E); // 398
    pub const SMSG_QUESTGIVER_QUEST_INVALID: Opcode = Opcode(0x018F); // 399
    pub const CMSG_QUESTGIVER_CANCEL: Opcode = Opcode(0x0190); // 400
    pub const SMSG_QUESTGIVER_QUEST_COMPLETE: Opcode = Opcode(0x0191); // 401
    pub const SMSG_QUESTGIVER_QUEST_FAILED: Opcode = Opcode(0x0192); // 402
    pub const CMSG_QUESTLOG_SWAP_QUEST: Opcode = Opcode(0x0193); // 403
    pub const CMSG_QUESTLOG_REMOVE_QUEST: Opcode = Opcode(0x0194); // 404
    pub const SMSG_QUESTLOG_FULL: Opcode = Opcode(0x0195); // 405
    pub const SMSG_QUESTUPDATE_FAILED: Opcode = Opcode(0x0196); // 406
    pub const SMSG_QUESTUPDATE_FAILEDTIMER: Opcode = Opcode(0x0197); // 407
    pub const SMSG_QUESTUPDATE_COMPLETE: Opcode = Opcode(0x0198); // 408
    pub const SMSG_QUESTUPDATE_ADD_KILL: Opcode = Opcode(0x0199); // 409
    pub const SMSG_QUESTUPDATE_ADD_ITEM: Opcode = Opcode(0x019A); // 410
    pub const CMSG_QUEST_CONFIRM_ACCEPT: Opcode = Opcode(0x019B); // 411
    pub const SMSG_QUEST_CONFIRM_ACCEPT: Opcode = Opcode(0x019C); // 412
    pub const CMSG_PUSHQUESTTOPARTY: Opcode = Opcode(0x019D); // 413

    // ============================================================================
    // Guild
    // ============================================================================

    pub const CMSG_GUILD_QUERY: Opcode = Opcode(0x0054); // 84
    pub const SMSG_GUILD_QUERY_RESPONSE: Opcode = Opcode(0x0055); // 85
    pub const CMSG_GUILD_CREATE: Opcode = Opcode(0x0081); // 129
    pub const CMSG_GUILD_INVITE: Opcode = Opcode(0x0082); // 130
    pub const SMSG_GUILD_INVITE: Opcode = Opcode(0x0083); // 131
    pub const CMSG_GUILD_ACCEPT: Opcode = Opcode(0x0084); // 132
    pub const CMSG_GUILD_DECLINE: Opcode = Opcode(0x0085); // 133
    pub const SMSG_GUILD_DECLINE: Opcode = Opcode(0x0086); // 134
    pub const CMSG_GUILD_INFO: Opcode = Opcode(0x0087); // 135
    pub const SMSG_GUILD_INFO: Opcode = Opcode(0x0088); // 136
    pub const CMSG_GUILD_ROSTER: Opcode = Opcode(0x0089); // 137
    pub const SMSG_GUILD_ROSTER: Opcode = Opcode(0x008A); // 138
    pub const CMSG_GUILD_PROMOTE: Opcode = Opcode(0x008B); // 139
    pub const CMSG_GUILD_DEMOTE: Opcode = Opcode(0x008C); // 140
    pub const CMSG_GUILD_LEAVE: Opcode = Opcode(0x008D); // 141
    pub const CMSG_GUILD_REMOVE: Opcode = Opcode(0x008E); // 142
    pub const CMSG_GUILD_DISBAND: Opcode = Opcode(0x008F); // 143
    pub const CMSG_GUILD_LEADER: Opcode = Opcode(0x0090); // 144
    pub const CMSG_GUILD_MOTD: Opcode = Opcode(0x0091); // 145
    pub const SMSG_GUILD_EVENT: Opcode = Opcode(0x0092); // 146
    pub const SMSG_GUILD_COMMAND_RESULT: Opcode = Opcode(0x0093); // 147
    pub const CMSG_GUILD_RANK: Opcode = Opcode(0x0231); // 561
    pub const CMSG_GUILD_ADD_RANK: Opcode = Opcode(0x0232); // 562
    pub const CMSG_GUILD_DEL_RANK: Opcode = Opcode(0x0233); // 563
    pub const CMSG_GUILD_SET_PUBLIC_NOTE: Opcode = Opcode(0x0234); // 564
    pub const CMSG_GUILD_SET_OFFICER_NOTE: Opcode = Opcode(0x0235); // 565
    pub const CMSG_GUILD_INFO_TEXT: Opcode = Opcode(0x02FC); // 764
    pub const MSG_SAVE_GUILD_EMBLEM: Opcode = Opcode(0x01F1); // 497

    // ============================================================================
    // Petition / Charter
    // ============================================================================

    pub const CMSG_PETITION_SHOWLIST: Opcode = Opcode(0x01BB); // 443
    pub const SMSG_PETITION_SHOWLIST: Opcode = Opcode(0x01BC); // 444
    pub const CMSG_PETITION_BUY: Opcode = Opcode(0x01BD); // 445
    pub const CMSG_PETITION_SHOW_SIGNATURES: Opcode = Opcode(0x01BE); // 446
    pub const SMSG_PETITION_SHOW_SIGNATURES: Opcode = Opcode(0x01BF); // 447
    pub const CMSG_PETITION_SIGN: Opcode = Opcode(0x01C0); // 448
    pub const SMSG_PETITION_SIGN_RESULTS: Opcode = Opcode(0x01C1); // 449
    pub const MSG_PETITION_DECLINE: Opcode = Opcode(0x01C2); // 450
    pub const SMSG_PETITION_QUERY_RESPONSE: Opcode = Opcode(0x01C3); // 451
    pub const CMSG_TURN_IN_PETITION: Opcode = Opcode(0x01C4); // 452
    pub const SMSG_TURN_IN_PETITION_RESULTS: Opcode = Opcode(0x01C5); // 453
    pub const CMSG_OFFER_PETITION: Opcode = Opcode(0x01C7); // 455
    pub const MSG_PETITION_RENAME: Opcode = Opcode(0x02C1); // 705

    // ============================================================================
    // Mail
    // ============================================================================

    pub const MSG_QUERY_NEXT_MAIL_TIME: Opcode = Opcode(0x0284); // 644
    pub const CMSG_SEND_MAIL: Opcode = Opcode(0x0238); // 568
    pub const SMSG_SEND_MAIL_RESULT: Opcode = Opcode(0x0239); // 569
    pub const CMSG_GET_MAIL_LIST: Opcode = Opcode(0x023A); // 570
    pub const SMSG_MAIL_LIST_RESULT: Opcode = Opcode(0x023B); // 571
    pub const CMSG_MAIL_TAKE_MONEY: Opcode = Opcode(0x0245); // 581
    pub const CMSG_MAIL_TAKE_ITEM: Opcode = Opcode(0x0246); // 582
    pub const CMSG_MAIL_MARK_AS_READ: Opcode = Opcode(0x0247); // 583
    pub const CMSG_MAIL_RETURN_TO_SENDER: Opcode = Opcode(0x0248); // 584
    pub const CMSG_MAIL_DELETE: Opcode = Opcode(0x0249); // 585
    pub const CMSG_MAIL_CREATE_TEXT_ITEM: Opcode = Opcode(0x024A); // 586
    pub const SMSG_RECEIVED_MAIL: Opcode = Opcode(0x0285); // 645

    // ============================================================================
    // Auction House
    // ============================================================================

    pub const MSG_AUCTION_HELLO: Opcode = Opcode(0x0255); // 597
    pub const CMSG_AUCTION_SELL_ITEM: Opcode = Opcode(0x0256); // 598
    pub const CMSG_AUCTION_REMOVE_ITEM: Opcode = Opcode(0x0257); // 599
    pub const CMSG_AUCTION_LIST_ITEMS: Opcode = Opcode(0x0258); // 600
    pub const CMSG_AUCTION_LIST_OWNER_ITEMS: Opcode = Opcode(0x0259); // 601
    pub const CMSG_AUCTION_PLACE_BID: Opcode = Opcode(0x025A); // 602
    pub const SMSG_AUCTION_COMMAND_RESULT: Opcode = Opcode(0x025B); // 603
    pub const SMSG_AUCTION_LIST_RESULT: Opcode = Opcode(0x025C); // 604
    pub const SMSG_AUCTION_OWNER_LIST_RESULT: Opcode = Opcode(0x025D); // 605
    pub const SMSG_AUCTION_BIDDER_NOTIFICATION: Opcode = Opcode(0x025E); // 606
    pub const SMSG_AUCTION_OWNER_NOTIFICATION: Opcode = Opcode(0x025F); // 607
    pub const CMSG_AUCTION_LIST_BIDDER_ITEMS: Opcode = Opcode(0x0264); // 612
    pub const SMSG_AUCTION_BIDDER_LIST_RESULT: Opcode = Opcode(0x0265); // 613
    pub const SMSG_AUCTION_REMOVED_NOTIFICATION: Opcode = Opcode(0x028D); // 653

    // ============================================================================
    // Battleground
    // ============================================================================

    pub const CMSG_BATTLEFIELD_STATUS: Opcode = Opcode(0x02D3); // 723
    pub const SMSG_BATTLEFIELD_STATUS: Opcode = Opcode(0x02D4); // 724
    pub const CMSG_BATTLEFIELD_LIST: Opcode = Opcode(0x023B); // 571
    pub const SMSG_BATTLEFIELD_LIST: Opcode = Opcode(0x023C); // 572
    pub const CMSG_BATTLEFIELD_JOIN: Opcode = Opcode(0x023E); // 574
    pub const SMSG_BATTLEFIELD_JOINED: Opcode = Opcode(0x02E1); // 737
    pub const SMSG_BATTLEFIELD_LEFT: Opcode = Opcode(0x02E2); // 738
    pub const CMSG_LEAVE_BATTLEFIELD: Opcode = Opcode(0x02E5); // 741
    pub const CMSG_BATTLEFIELD_PORT: Opcode = Opcode(0x02D5); // 725
    pub const CMSG_BATTLEMASTER_HELLO: Opcode = Opcode(0x02D7); // 727
    pub const SMSG_BATTLEMASTER_JOINED: Opcode = Opcode(0x02E3); // 739
    pub const CMSG_BATTLEFIELD_QUEUE: Opcode = Opcode(0x023D); // 573
    pub const CMSG_BATTLEFIELD_UN_QUEUE: Opcode = Opcode(0x023F); // 575
    pub const CMSG_AREA_SPIRIT_HEALER_QUERY: Opcode = Opcode(0x02E2); // 738
    pub const CMSG_AREA_SPIRIT_HEALER_QUEUE: Opcode = Opcode(0x02E3); // 739
    pub const SMSG_AREA_SPIRIT_HEALER_TIME: Opcode = Opcode(0x02E4); // 740

    // ============================================================================
    // Instance & Raid
    // ============================================================================

    pub const CMSG_REQUEST_RAID_INFO: Opcode = Opcode(0x02CD); // 717
    pub const SMSG_RAID_INSTANCE_INFO: Opcode = Opcode(0x02CC); // 716
    pub const CMSG_RESET_INSTANCES: Opcode = Opcode(0x031D); // 797
    pub const SMSG_INSTANCE_RESET: Opcode = Opcode(0x031E); // 798
    pub const SMSG_INSTANCE_RESET_FAILED: Opcode = Opcode(0x031F); // 799

    // ============================================================================
    // Meeting Stone
    // ============================================================================

    pub const CMSG_MEETINGSTONE_INFO: Opcode = Opcode(0x0296); // 662
    pub const CMSG_MEETINGSTONE_JOIN: Opcode = Opcode(0x0292); // 658
    pub const CMSG_MEETINGSTONE_LEAVE: Opcode = Opcode(0x0293); // 659
    pub const CMSG_MEETINGSTONE_CHEAT: Opcode = Opcode(0x0294); // 660
    pub const SMSG_MEETINGSTONE_SETQUEUE: Opcode = Opcode(0x0295); // 661

    // ============================================================================
    // Duel
    // ============================================================================

    pub const CMSG_DUEL_ACCEPTED: Opcode = Opcode(0x016C); // 364
    pub const CMSG_DUEL_CANCELLED: Opcode = Opcode(0x016D); // 365
    pub const SMSG_DUEL_REQUESTED: Opcode = Opcode(0x0167); // 359
    pub const SMSG_DUEL_COUNTDOWN: Opcode = Opcode(0x02B7); // 695
    pub const SMSG_DUEL_OUTOFBOUNDS: Opcode = Opcode(0x0168); // 360
    pub const SMSG_DUEL_INBOUNDS: Opcode = Opcode(0x0169); // 361
    pub const SMSG_DUEL_COMPLETE: Opcode = Opcode(0x016A); // 362
    pub const SMSG_DUEL_WINNER: Opcode = Opcode(0x016B); // 363

    // ============================================================================
    // Pet
    // ============================================================================

    pub const CMSG_PET_NAME_QUERY: Opcode = Opcode(0x0052); // 82
    pub const SMSG_PET_NAME_QUERY_RESPONSE: Opcode = Opcode(0x0053); // 83
    pub const CMSG_PET_SET_ACTION: Opcode = Opcode(0x0174); // 372
    pub const CMSG_PET_ACTION: Opcode = Opcode(0x0175); // 373
    pub const CMSG_PET_ABANDON: Opcode = Opcode(0x0176); // 374
    pub const CMSG_PET_RENAME: Opcode = Opcode(0x0177); // 375
    pub const SMSG_PET_SPELLS: Opcode = Opcode(0x0179); // 377
    pub const SMSG_PET_MODE: Opcode = Opcode(0x017A); // 378
    pub const SMSG_PET_TAME_FAILURE: Opcode = Opcode(0x0173); // 371
    pub const SMSG_PET_NAME_INVALID: Opcode = Opcode(0x0178); // 376
    pub const CMSG_PET_CAST_SPELL: Opcode = Opcode(0x01F0); // 496
    pub const SMSG_PET_CAST_FAILED: Opcode = Opcode(0x0138); // 312
    pub const CMSG_PET_CANCEL_AURA: Opcode = Opcode(0x026B); // 619
    pub const SMSG_PET_ACTION_FEEDBACK: Opcode = Opcode(0x02C6); // 710
    pub const SMSG_PET_BROKEN: Opcode = Opcode(0x02B3); // 691
    pub const CMSG_PET_UNLEARN: Opcode = Opcode(0x02F0); // 752
    pub const SMSG_PET_UNLEARN_CONFIRM: Opcode = Opcode(0x02F1); // 753
    pub const CMSG_PET_SPELL_AUTOCAST: Opcode = Opcode(0x02F3); // 755
    pub const CMSG_PET_STOP_ATTACK: Opcode = Opcode(0x02EA); // 746
    pub const CMSG_REQUEST_PET_INFO: Opcode = Opcode(0x0279); // 633

    // ============================================================================
    // Pet Stable
    // ============================================================================

    pub const MSG_LIST_STABLED_PETS: Opcode = Opcode(0x026E); // 623
    pub const CMSG_STABLE_PET: Opcode = Opcode(0x026F); // 624
    pub const CMSG_UNSTABLE_PET: Opcode = Opcode(0x0270); // 625
    pub const CMSG_BUY_STABLE_SLOT: Opcode = Opcode(0x0272); // 626
    pub const SMSG_STABLE_RESULT: Opcode = Opcode(0x0273); // 627
    pub const CMSG_STABLE_REVIVE_PET: Opcode = Opcode(0x0274); // 628
    pub const CMSG_STABLE_SWAP_PET: Opcode = Opcode(0x0275); // 629

    // ============================================================================
    // GM Ticket
    // ============================================================================

    pub const CMSG_GMTICKET_CREATE: Opcode = Opcode(0x0205); // 517
    pub const SMSG_GMTICKET_CREATE: Opcode = Opcode(0x0206); // 518
    pub const CMSG_GMTICKET_UPDATETEXT: Opcode = Opcode(0x0207); // 519
    pub const SMSG_GMTICKET_UPDATETEXT: Opcode = Opcode(0x0208); // 520
    pub const CMSG_GMTICKET_GETTICKET: Opcode = Opcode(0x0211); // 529
    pub const SMSG_GMTICKET_GETTICKET: Opcode = Opcode(0x0212); // 530
    pub const CMSG_GMTICKET_DELETETICKET: Opcode = Opcode(0x0217); // 535
    pub const SMSG_GMTICKET_DELETETICKET: Opcode = Opcode(0x0218); // 536
    pub const CMSG_GMTICKET_SYSTEMSTATUS: Opcode = Opcode(0x021A); // 538
    pub const SMSG_GMTICKET_SYSTEMSTATUS: Opcode = Opcode(0x021B); // 539
    pub const CMSG_GMSURVEY_SUBMIT: Opcode = Opcode(0x032A); // 810

    // ============================================================================
    // PvP
    // ============================================================================

    pub const CMSG_TOGGLE_PVP: Opcode = Opcode(0x0253); // 595

    // ============================================================================
    // Summon
    // ============================================================================

    pub const SMSG_SUMMON_REQUEST: Opcode = Opcode(0x02AB); // 683
    pub const CMSG_SUMMON_RESPONSE: Opcode = Opcode(0x02AC); // 684

    // ============================================================================
    // Far Sight
    // ============================================================================

    pub const CMSG_FAR_SIGHT: Opcode = Opcode(0x027A); // 634

    // ============================================================================
    // Appearance
    // ============================================================================

    pub const CMSG_TOGGLE_HELM: Opcode = Opcode(0x02B9); // 697
    pub const CMSG_TOGGLE_CLOAK: Opcode = Opcode(0x02BA); // 698

    // ============================================================================
    // Player Misc
    // ============================================================================

    pub const CMSG_SAVE_PLAYER: Opcode = Opcode(0x0153); // 339
    pub const CMSG_SETSHEATHED: Opcode = Opcode(0x01E0); // 480
    pub const CMSG_GHOST: Opcode = Opcode(0x01E5); // 485
    pub const CMSG_PLAYED_TIME: Opcode = Opcode(0x01CC); // 460
    pub const SMSG_PLAYED_TIME: Opcode = Opcode(0x01CD); // 461
    pub const CMSG_BUG: Opcode = Opcode(0x01CA); // 458

    // ============================================================================
    // Warden (Anticheat)
    // ============================================================================

    pub const CMSG_WARDEN_DATA: Opcode = Opcode(0x02E7); // 743
    pub const SMSG_WARDEN_DATA: Opcode = Opcode(0x02E6); // 742

    // ============================================================================
    // Weather
    // ============================================================================

    pub const SMSG_WEATHER: Opcode = Opcode(0x02F4); // 756
}

impl From<u16> for Opcode {
    fn from(value: u16) -> Self {
        Self(value as u32)
    }
}

impl From<u32> for Opcode {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
