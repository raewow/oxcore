use crate::auth::realm::build_info::RealmBuildInfo;

/// Maximum number of realm zones
const MAX_REALM_ZONES: usize = 38;

/// Realm category IDs by realm zone by major version
/// [major_version][realm_zone] = category_id
static REALM_CATEGORY_IDS: [[u8; MAX_REALM_ZONES]; 4] = [
    // 0 - Alpha
    [
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ],
    // 1 - Classic
    [
        0, 1, 1, 5, 1, 1, 1, 1, 1, 2, 3, 5, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ],
    // 2 - TBC
    [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 0, 0, 0, 0, 0, 0, 0,
    ],
    // 3 - WotLK
    [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
    ],
];

/// Get realm category ID by build and zone
/// Matches the C++ GetRealmCategoryIdByBuildAndZone function
pub fn get_realm_category_id_by_build_and_zone(
    _build: u16,
    realm_zone: u8,
    build_info: Option<&RealmBuildInfo>,
) -> u8 {
    let realm_zone = if realm_zone >= MAX_REALM_ZONES as u8 {
        0 // REALM_ZONE_DEVELOPMENT
    } else {
        realm_zone
    };

    if let Some(build_info) = build_info {
        if build_info.major_version < 4 {
            let major = build_info.major_version as usize;
            if major < REALM_CATEGORY_IDS.len() {
                let zone = realm_zone as usize;
                if zone < MAX_REALM_ZONES {
                    return REALM_CATEGORY_IDS[major][zone];
                }
            }
        }
    }

    realm_zone
}
