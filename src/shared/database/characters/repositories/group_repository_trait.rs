use super::super::models::group::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait abstraction for group repository operations.
/// Enables dependency injection and mocking for tests.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait GroupRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum group ID from the database (for generating next ID).
    async fn get_max_group_id(&self) -> Result<Option<u32>>;

    /// Find a group by ID.
    async fn find_by_id(&self, group_id: u32) -> Result<Option<GroupRow>>;

    /// Load all groups from the database.
    async fn find_all(&self) -> Result<Vec<GroupRow>>;

    /// Find all members for a group.
    async fn find_members(&self, group_id: u32) -> Result<Vec<GroupMemberRow>>;

    /// Find group ID for a member.
    async fn find_group_for_member(&self, member_guid: u32) -> Result<Option<u32>>;

    /// Find members with character data (LEFT JOIN).
    async fn find_members_with_character_data(
        &self,
        group_id: u32,
    ) -> Result<Vec<GroupMemberWithCharacterDataRow>>;

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create or update a group (uses REPLACE INTO).
    async fn save_group(&self, group: &GroupRow) -> Result<()>;

    /// Add a member to a group.
    async fn add_member(&self, group_id: u32, member_guid: u32, subgroup: u16) -> Result<()>;

    /// Update member subgroup or assistant status.
    async fn update_member(
        &self,
        group_id: u32,
        member_guid: u32,
        assistant: bool,
        subgroup: u16,
    ) -> Result<()>;

    /// Remove a member from a group.
    async fn remove_member(&self, group_id: u32, member_guid: u32) -> Result<()>;

    // ========== DELETE OPERATIONS ==========

    /// Delete a group and all its members (transactional).
    async fn delete_group(&self, group_id: u32) -> Result<()>;
}
