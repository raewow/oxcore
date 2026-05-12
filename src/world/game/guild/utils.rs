//! Guild system utility functions

use super::types::*;

/// Calculate rank permissions mask
pub fn calc_rank_permissions() -> u32 {
    // Full permissions for Guild Master
    0x7FFFFFFF
}

/// Create default ranks for a new guild
pub fn create_default_ranks() -> Vec<GuildRank> {
    vec![
        GuildRank {
            id: 0,
            name: "Guild Master".to_string(),
            rights: calc_rank_permissions(),
        },
        GuildRank {
            id: 1,
            name: "Officer".to_string(),
            rights: 0x00000040, // GRIGHT_GCHATLISTEN
        },
        GuildRank {
            id: 2,
            name: "Veteran".to_string(),
            rights: 0x00000010, // GRIGHT_OFFCHATLISTEN
        },
        GuildRank {
            id: 3,
            name: "Member".to_string(),
            rights: 0x00000010,
        },
        GuildRank {
            id: 4,
            name: "Initiate".to_string(),
            rights: 0x00000010,
        },
    ]
}

/// Create default guild emblem
pub fn create_default_emblem() -> GuildEmblem {
    GuildEmblem::default()
}
