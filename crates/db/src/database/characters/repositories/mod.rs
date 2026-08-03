pub mod auction_repository;
pub mod auction_repository_trait;
pub mod character_repository;
pub mod corpse_repository;
pub mod group_repository;
pub mod group_repository_trait;
pub mod guild_repository;
pub mod honor_repository;
pub mod inventory_repository;
pub mod inventory_repository_trait;
pub mod item_repository;
pub mod item_repository_trait;
pub mod mail_repository;
pub mod mail_repository_trait;
pub mod quest_repository;
pub mod social_repository;
pub mod social_repository_trait;
pub mod ticket_repository;

// Re-export for convenience
pub use auction_repository::AuctionRepository;
pub use auction_repository_trait::AuctionRepositoryTrait;
pub use character_repository::{
    CharacterDeleteMode, CharacterRepository, CharacterRepositoryTrait,
};
pub use corpse_repository::CorpseRepository;
pub use group_repository::GroupRepository;
pub use group_repository_trait::GroupRepositoryTrait;
pub use guild_repository::{GuildRepository, GuildRepositoryTrait};
pub use honor_repository::HonorRepository;
pub use inventory_repository::InventoryRepository;
pub use inventory_repository_trait::InventoryRepositoryTrait;
pub use inventory_repository_trait::InventorySlotRow;
#[cfg(any(test, feature = "testing"))]
pub use inventory_repository_trait::MockInventoryRepositoryTrait;
pub use item_repository::ItemRepository;
pub use item_repository_trait::ItemRepositoryTrait;
#[cfg(any(test, feature = "testing"))]
pub use item_repository_trait::MockItemRepositoryTrait;
pub use mail_repository::MailRepository;
pub use mail_repository_trait::MailRepositoryTrait;
#[cfg(any(test, feature = "testing"))]
pub use quest_repository::MockQuestRepositoryTrait;
pub use quest_repository::{QuestRepository, QuestRepositoryTrait};
pub use social_repository::SocialRepository;
pub use social_repository_trait::SocialRepositoryTrait;
pub use ticket_repository::TicketRepository;
