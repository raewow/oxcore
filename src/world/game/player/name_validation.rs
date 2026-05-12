use crate::world::config::get_config_mgr;
use rustrict::CensorStr;

/// Character name validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameValidationResult {
    Success,
    TooShort,
    TooLong,
    InvalidCharacters,
    Profane,
    Reserved,
    MixedLanguages,
}

/// Validate a character name according to server configuration
pub fn validate_character_name(name: &str) -> NameValidationResult {
    let config = get_config_mgr().get();

    let name_len = name.chars().count() as u32;
    if name_len < config.min_player_name {
        return NameValidationResult::TooShort;
    }
    if name_len > config.max_player_name {
        return NameValidationResult::TooLong;
    }

    if name.is_inappropriate() {
        return NameValidationResult::Profane;
    }

    if !name
        .chars()
        .all(|c| c.is_alphabetic() || c == ' ' || c == '-' || c == '\'')
    {
        return NameValidationResult::InvalidCharacters;
    }

    if config.strict_player_names != 0 {
        let has_latin = name.chars().any(|c| c.is_ascii_alphabetic());
        let has_non_latin = name.chars().any(|c| !c.is_ascii() && c.is_alphabetic());

        if has_latin && has_non_latin {
            return NameValidationResult::MixedLanguages;
        }
    }

    NameValidationResult::Success
}

/// Validate a pet name
pub fn validate_pet_name(name: &str) -> Result<(), NameValidationResult> {
    let config = get_config_mgr().get();

    let name_len = name.chars().count() as u32;
    if name_len < 2 {
        return Err(NameValidationResult::TooShort);
    }
    if name_len > 21 {
        return Err(NameValidationResult::TooLong);
    }

    if name.is_inappropriate() {
        return Err(NameValidationResult::Profane);
    }

    if !name
        .chars()
        .all(|c| c.is_alphabetic() || c == ' ' || c == '-' || c == '\'')
    {
        return Err(NameValidationResult::InvalidCharacters);
    }

    if config.strict_player_names != 0 {
        let has_latin = name.chars().any(|c| c.is_ascii_alphabetic());
        let has_non_latin = name.chars().any(|c| !c.is_ascii() && c.is_alphabetic());

        if has_latin && has_non_latin {
            return Err(NameValidationResult::MixedLanguages);
        }
    }

    Ok(())
}

/// Normalize a character name (trim, capitalize first letter)
pub fn normalize_character_name(name: &str) -> String {
    let trimmed = name.trim();

    let mut result = String::with_capacity(trimmed.len());
    let mut capitalize_next = true;

    for c in trimmed.chars() {
        if c.is_whitespace() {
            result.push(c);
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else {
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
    }

    result
}

/// Normalize player name for lookup (trim, lowercase)
pub fn normalize_player_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Check if a name is reserved
pub fn is_reserved_name(_name: &str) -> bool {
    false
}
