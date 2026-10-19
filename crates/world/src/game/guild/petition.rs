//! Guild charters: bought from a guild master, signed by nine other players of the same side,
//! and turned in to found the guild.
//!
//! Charters live in memory and are written to `characters.petition` / `petition_sign` in the
//! background; the charter item carries its petition id in its first enchantment slot.

use dashmap::DashMap;
use oxcore_db::database::characters::postgres::{
    PgPetitionRepository, PgPetitionRow, PgPetitionSignatureRow,
};
use oxcore_shared::game::chat::Team;
use oxcore_shared::game::petition::{PetitionSignature, GUILD_CHARTER_SIGNATURES};
use oxcore_shared::protocol::ObjectGuid;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

/// The guild charter item.
pub const GUILD_CHARTER_ITEM: u32 = 5863;
/// What a guild charter costs, in copper.
pub const GUILD_CHARTER_COST: u32 = 1000;

/// One guild charter.
#[derive(Debug, Clone)]
pub struct Petition {
    pub id: u32,
    pub owner: ObjectGuid,
    pub charter: ObjectGuid,
    pub name: String,
    /// Only players of the owner's side may sign.
    pub team: Team,
    pub signatures: Vec<PetitionSignature>,
}

impl Petition {
    pub fn is_complete(&self) -> bool {
        self.signatures.len() as u32 >= GUILD_CHARTER_SIGNATURES
    }

    pub fn signed_by(&self, player: ObjectGuid, account: u32) -> bool {
        self.signatures
            .iter()
            .any(|s| s.player_guid == player || (account != 0 && s.player_account == account))
    }
}

/// Every charter in the world.
pub struct Petitions {
    petitions: DashMap<u32, Petition>,
    next_id: AtomicU32,
    repository: OnceLock<Arc<PgPetitionRepository>>,
}

impl Default for Petitions {
    fn default() -> Self {
        Self {
            petitions: DashMap::new(),
            next_id: AtomicU32::new(1),
            repository: OnceLock::new(),
        }
    }
}

impl Petitions {
    /// Load the charters stored in `pool`, and write changes back there from now on.
    pub async fn load(&self, pool: &PgPool) -> anyhow::Result<()> {
        let repository = Arc::new(PgPetitionRepository::new(Arc::new(pool.clone())));
        for (row, race) in repository.load_all_petitions().await? {
            let Some(charter) = row.charter_guid else {
                continue;
            };
            let id = u32::try_from(row.petition_guid)?;
            self.petitions.insert(
                id,
                Petition {
                    id,
                    owner: ObjectGuid::new_player(u32::try_from(row.owner_guid)?),
                    charter: ObjectGuid::new_item(u32::try_from(charter)?),
                    name: row.name,
                    team: Team::from_race(u8::try_from(race)?),
                    signatures: Vec::new(),
                },
            );
        }
        for row in repository.load_all_signatures().await? {
            if let Some(mut petition) = self.petitions.get_mut(&u32::try_from(row.petition_guid)?) {
                petition.signatures.push(PetitionSignature {
                    player_guid: ObjectGuid::new_player(u32::try_from(row.player_guid)?),
                    player_account: u32::try_from(row.player_account)?,
                });
            }
        }
        let highest = self.petitions.iter().map(|p| p.id).max().unwrap_or(0);
        self.next_id.store(highest + 1, Ordering::Relaxed);
        let _ = self.repository.set(repository);
        tracing::info!("Loaded {} guild charters", self.petitions.len());
        Ok(())
    }

    pub fn get(&self, id: u32) -> Option<Petition> {
        self.petitions.get(&id).map(|p| p.clone())
    }

    pub fn by_owner(&self, owner: ObjectGuid) -> Option<Petition> {
        self.petitions
            .iter()
            .find(|p| p.owner == owner)
            .map(|p| p.clone())
    }

    pub fn by_charter(&self, charter: ObjectGuid) -> Option<Petition> {
        self.petitions
            .iter()
            .find(|p| p.charter == charter)
            .map(|p| p.clone())
    }

    /// Start a charter for `owner`, carried as item `charter`. Returns its id.
    pub fn create(&self, owner: ObjectGuid, charter: ObjectGuid, name: &str, team: Team) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let petition = Petition {
            id,
            owner,
            charter,
            name: name.to_string(),
            team,
            signatures: Vec::new(),
        };
        let row = PgPetitionRow {
            owner_guid: i64::from(owner.counter()),
            petition_guid: i64::from(id),
            charter_guid: Some(i64::from(charter.counter())),
            name: petition.name.clone(),
        };
        self.petitions.insert(id, petition);
        self.persist(move |repo| async move { repo.save_petition(&row).await });
        id
    }

    pub fn rename(&self, id: u32, name: &str) -> bool {
        let Some(mut petition) = self.petitions.get_mut(&id) else {
            return false;
        };
        petition.name = name.to_string();
        let name = name.to_string();
        self.persist(move |repo| async move { repo.rename_petition(i64::from(id), &name).await });
        true
    }

    pub fn add_signature(&self, id: u32, signature: PetitionSignature) -> bool {
        let Some(mut petition) = self.petitions.get_mut(&id) else {
            return false;
        };
        let row = PgPetitionSignatureRow {
            owner_guid: i64::from(petition.owner.counter()),
            petition_guid: i64::from(id),
            player_guid: i64::from(signature.player_guid.counter()),
            player_account: i64::from(signature.player_account),
        };
        petition.signatures.push(signature);
        self.persist(move |repo| async move { repo.add_signature(&row).await });
        true
    }

    /// Forget a charter and its signatures.
    pub fn remove(&self, id: u32) {
        let Some((_, petition)) = self.petitions.remove(&id) else {
            return;
        };
        let charter = i64::from(petition.charter.counter());
        self.persist(move |repo| async move { repo.delete_petition(charter).await });
    }

    fn persist<F, Fut>(&self, write: F)
    where
        F: FnOnce(Arc<PgPetitionRepository>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send,
    {
        let Some(repository) = self.repository.get().cloned() else {
            return;
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = write(repository).await {
                    tracing::error!("Failed to save guild charter: {e:#}");
                }
            });
        }
    }
}

/// Whether a name can go on a charter: 2 to 24 letters and single spaces.
pub fn is_valid_charter_name(name: &str) -> bool {
    let length = name.chars().count();
    (2..=oxcore_shared::game::guild::GUILD_NAME_MAX_LENGTH).contains(&length)
        && name.chars().all(|c| c.is_alphabetic() || c == ' ')
        && !name.starts_with(' ')
        && !name.ends_with(' ')
        && !name.contains("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charter_names_are_letters_and_spaces() {
        assert!(is_valid_charter_name("Knights of Dawn"));
        assert!(!is_valid_charter_name("K"));
        assert!(!is_valid_charter_name("Guild2"));
        assert!(!is_valid_charter_name(" Lead"));
        assert!(!is_valid_charter_name("Two  Spaces"));
    }

    #[test]
    fn nine_signatures_complete_a_charter() {
        let book = Petitions::default();
        let owner = ObjectGuid::new_player(1);
        let id = book.create(owner, ObjectGuid::new_item(5), "Dawn", Team::Alliance);
        for signer in 2..10 {
            book.add_signature(
                id,
                PetitionSignature {
                    player_guid: ObjectGuid::new_player(signer),
                    player_account: signer,
                },
            );
            assert!(!book.get(id).unwrap().is_complete());
        }
        book.add_signature(
            id,
            PetitionSignature {
                player_guid: ObjectGuid::new_player(10),
                player_account: 10,
            },
        );
        let petition = book.get(id).unwrap();
        assert!(petition.is_complete());
        assert!(petition.signed_by(ObjectGuid::new_player(3), 0));
        assert!(petition.signed_by(ObjectGuid::new_player(99), 4));
        assert_eq!(book.by_owner(owner).map(|p| p.id), Some(id));
    }
}
