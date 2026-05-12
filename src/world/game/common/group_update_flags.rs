pub const NONE: u32 = 0x00000000;
pub const STATUS: u32 = 0x00000001;
pub const CUR_HP: u32 = 0x00000002;
pub const MAX_HP: u32 = 0x00000004;
pub const POWER_TYPE: u32 = 0x00000008;
pub const CUR_POWER: u32 = 0x00000010;
pub const MAX_POWER: u32 = 0x00000020;
pub const LEVEL: u32 = 0x00000040;
pub const ZONE: u32 = 0x00000080;
pub const POSITION: u32 = 0x00000100;
pub const AURAS: u32 = 0x00000200;
pub const AURAS_NEGATIVE: u32 = 0x00000400;
pub const PET_GUID: u32 = 0x00000800;
pub const PET_NAME: u32 = 0x00001000;
pub const PET_MODEL_ID: u32 = 0x00002000;
pub const PET_CUR_HP: u32 = 0x00004000;
pub const PET_MAX_HP: u32 = 0x00008000;
pub const PET_POWER_TYPE: u32 = 0x00010000;
pub const PET_CUR_POWER: u32 = 0x00020000;
pub const PET_MAX_POWER: u32 = 0x00040000;
pub const PET_AURAS: u32 = 0x00080000;
pub const PET_AURAS_NEGATIVE: u32 = 0x00100000;

pub const PET: u32 = 0x001FF800;
pub const FULL: u32 = 0x001FFFFF;

pub const UPDATE_LENGTH: [usize; 21] = [
    1, 2, 2, 1, 2, 2, 2, 2, 4, 4, 2, 8, 1, 2, 2, 2, 1, 2, 2, 4, 2,
];
