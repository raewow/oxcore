//! Chat message validation utilities
//!
//! This module provides validation functions for chat messages including:
//! - Message length validation
//! - Character validation
//! - Channel name validation
//! - Language validation

use super::types::*;
use crate::shared::game::chat::{ChatMsg, Language, Team};

/// Validate a chat message
///
/// Returns Ok(()) if the message is valid, or an appropriate ChatError
pub fn validate_message(message: &str) -> Result<(), ChatError> {
    if message.is_empty() || message.trim().is_empty() {
        return Err(ChatError::EmptyMessage);
    }

    if message.len() > MAX_MESSAGE_LENGTH {
        return Err(ChatError::MessageTooLong);
    }

    if has_invalid_chars(message) {
        return Err(ChatError::InvalidCharacters);
    }

    Ok(())
}

/// Check if message contains invalid control characters
fn has_invalid_chars(message: &str) -> bool {
    for c in message.chars() {
        if c.is_ascii_control() && c != '\t' && c != '\n' {
            return true;
        }
        if c == '\0' {
            return true;
        }
    }
    false
}

/// Strip invisible/control characters from a message
///
/// Removes control characters except newlines and tabs.
/// This is less strict than validation - it sanitizes rather than rejects.
pub fn strip_invisible_chars(message: &str) -> String {
    message
        .chars()
        .filter(|&c| c.is_ascii_graphic() || c == '\n' || c == '\t' || c == ' ')
        .collect()
}

/// Check if message contains only Latin (ASCII) characters
pub fn is_latin_only(message: &str) -> bool {
    message.chars().all(|c| c.is_ascii())
}

/// Validate message length against a maximum
pub fn validate_message_length(message: &str, max_length: usize) -> bool {
    message.len() <= max_length
}

/// Validate channel name
pub fn validate_channel_name(name: &str) -> Result<(), ChatError> {
    if name.is_empty() || name.trim().is_empty() {
        return Err(ChatError::InvalidChannelName);
    }

    if name.len() > MAX_CHANNEL_NAME_LENGTH {
        return Err(ChatError::InvalidChannelName);
    }

    for c in name.chars() {
        if !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_' {
            return Err(ChatError::InvalidChannelName);
        }
    }

    Ok(())
}

/// Check if a language is valid for a given chat type
pub fn is_valid_language_for_chat_type(language: Language, chat_type: ChatMsg) -> bool {
    match chat_type {
        ChatMsg::Addon => language == Language::Addon,
        ChatMsg::System => language == Language::Universal,
        ChatMsg::Emote | ChatMsg::TextEmote => language == Language::Universal,
        _ => true,
    }
}

/// Validate item links in a message (placeholder)
///
/// In a full implementation, this would validate that item links
/// reference real items and have correct formats.
pub fn validate_links(_message: &str) -> bool {
    true
}

/// Normalize a channel name for comparison
pub fn normalize_channel_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Check if a player can speak a language
///
/// This is a simplified check - a full implementation would check:
/// - Player's learned languages from skills
/// - Race-specific languages
/// - GM language bypass
pub fn can_speak_language(language: Language, race: u8, is_gm: bool) -> bool {
    if is_gm {
        return true;
    }

    if language == Language::Universal || language == Language::Addon {
        return true;
    }

    let team = Team::from_race(race);

    match language {
        Language::Common => team == Team::Alliance,
        Language::Darnassian => race == 4,
        Language::Dwarvish => race == 3,
        Language::Gnomish => race == 7,
        Language::Orcish => team == Team::Horde,
        Language::Taurahe => race == 6,
        Language::Troll => race == 8,
        Language::Gutterspeak => race == 5,
        Language::Demonic | Language::Titan | Language::Draconic | Language::Kalimag => false,
        Language::Thalassian => false,
        _ => false,
    }
}

/// Check if a language can be understood by a player
///
/// Used for cross-faction communication checks
pub fn can_understand_language(listener_race: u8, speaker_language: Language, is_gm: bool) -> bool {
    if is_gm {
        return true;
    }

    if speaker_language == Language::Universal {
        return true;
    }

    can_speak_language(speaker_language, listener_race, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_message_empty() {
        assert_eq!(validate_message(""), Err(ChatError::EmptyMessage));
        assert_eq!(validate_message("   "), Err(ChatError::EmptyMessage));
    }

    #[test]
    fn test_validate_message_too_long() {
        let long_msg = "a".repeat(MAX_MESSAGE_LENGTH + 1);
        assert_eq!(validate_message(&long_msg), Err(ChatError::MessageTooLong));

        let ok_msg = "a".repeat(MAX_MESSAGE_LENGTH);
        assert!(validate_message(&ok_msg).is_ok());
    }

    #[test]
    fn test_validate_message_valid() {
        assert!(validate_message("Hello, world!").is_ok());
        assert!(validate_message("Testing 123").is_ok());
        assert!(validate_message("Multi\nline").is_ok());
    }

    #[test]
    fn test_strip_invisible_chars() {
        assert_eq!(strip_invisible_chars("Hello\x00World"), "HelloWorld");
        assert_eq!(strip_invisible_chars("Test\t123"), "Test\t123");
        assert_eq!(strip_invisible_chars("Line1\nLine2"), "Line1\nLine2");
    }

    #[test]
    fn test_validate_channel_name() {
        assert!(validate_channel_name("MyChannel").is_ok());
        assert!(validate_channel_name("My Channel").is_ok());
        assert!(validate_channel_name("Channel-123").is_ok());

        assert_eq!(
            validate_channel_name(""),
            Err(ChatError::InvalidChannelName)
        );
        assert_eq!(
            validate_channel_name("   "),
            Err(ChatError::InvalidChannelName)
        );

        let long_name = "a".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        assert_eq!(
            validate_channel_name(&long_name),
            Err(ChatError::InvalidChannelName)
        );
    }

    #[test]
    fn test_is_valid_language_for_chat_type() {
        assert!(is_valid_language_for_chat_type(
            Language::Addon,
            ChatMsg::Addon
        ));
        assert!(!is_valid_language_for_chat_type(
            Language::Common,
            ChatMsg::Addon
        ));

        assert!(is_valid_language_for_chat_type(
            Language::Universal,
            ChatMsg::System
        ));
        assert!(!is_valid_language_for_chat_type(
            Language::Common,
            ChatMsg::System
        ));

        assert!(is_valid_language_for_chat_type(
            Language::Common,
            ChatMsg::Say
        ));
        assert!(is_valid_language_for_chat_type(
            Language::Orcish,
            ChatMsg::Yell
        ));
    }

    #[test]
    fn test_can_speak_language() {
        assert!(can_speak_language(Language::Common, 1, false));
        assert!(!can_speak_language(Language::Common, 2, false));
        assert!(can_speak_language(Language::Orcish, 2, false));
        assert!(can_speak_language(Language::Universal, 1, false));
        assert!(can_speak_language(Language::Universal, 2, false));
        assert!(can_speak_language(Language::Demonic, 1, true));
    }

    #[test]
    fn test_normalize_channel_name() {
        assert_eq!(normalize_channel_name("General"), "general");
        assert_eq!(normalize_channel_name("  Trade  "), "trade");
        assert_eq!(normalize_channel_name("MyChannel"), "mychannel");
    }
}
