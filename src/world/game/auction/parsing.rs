//! Shared field parsing for auction item rows (matches inventory semantics).

/// Parse space-separated enchantment triplets from DB text.
pub fn parse_enchantments(enchantments_str: &str) -> Vec<(u32, u32, u32)> {
    let values: Vec<u32> = enchantments_str
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut enchantments = Vec::new();
    for i in (0..values.len()).step_by(3) {
        if i + 2 < values.len() {
            enchantments.push((values[i], values[i + 1], values[i + 2]));
        }
    }
    enchantments
}

/// Parse up to five spell charges from optional DB text.
pub fn parse_spell_charges(charges_str: Option<&str>) -> [i32; 5] {
    let mut charges = [0i32; 5];
    if let Some(s) = charges_str {
        let values: Vec<i32> = s
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        for (i, value) in values.iter().enumerate().take(5) {
            charges[i] = *value;
        }
    }
    charges
}
