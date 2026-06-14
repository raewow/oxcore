//! Auction House Manager tests

use super::manager::AuctionHouseManager;
use super::parsing::{parse_enchantments, parse_spell_charges};
use crate::shared::database::characters::models::auction::AuctionRow;
use crate::shared::database::characters::repositories::auction_repository_trait::MockAuctionRepositoryTrait;
use crate::shared::database::characters::repositories::mail_repository_trait::MockMailRepositoryTrait;
use crate::shared::database::characters::repositories::CharacterRepository;
use crate::shared::protocol::{HighGuid, ObjectGuid};
use crate::world::dbc::manager::DbcManager;
use crate::world::dbc::structures::AuctionHouseEntry;
use crate::world::game::items::manager::ItemTemplate;
use crate::world::game::items::Item;
use crate::world::game::ItemManager;
use mockall::predicate::*;
use parking_lot::RwLock;
use std::sync::Arc;

fn test_item_template(entry: u32) -> ItemTemplate {
    ItemTemplate {
        entry,
        name: format!("Test Item {entry}"),
        display_id: 0,
        quality: 0,
        item_level: 1,
        required_level: 1,
        item_class: 0,
        item_subclass: 0,
        inventory_type: 0,
        max_count: 0,
        stackable: 1,
        max_durability: 100,
        buy_price: 0,
        sell_price: 0,
        container_slots: 0,
        start_quest: 0,
        spell_id: [0; 5],
        spell_trigger: [0; 5],
        spell_charges: [0; 5],
        spell_cooldown: [0; 5],
        spell_category: [0; 5],
        spell_category_cooldown: [0; 5],
    }
}

fn test_item(guid_low: u32, entry: u32) -> Arc<Item> {
    test_item_with_count(guid_low, entry, 1)
}

fn test_item_with_count(guid_low: u32, entry: u32, count: u32) -> Arc<Item> {
    Arc::new(Item::from_db_row(
        ObjectGuid::new_without_entry(HighGuid::Item, guid_low),
        entry,
        count,
        ObjectGuid::empty(),
        0,
        0,
        0,
        50,
        100,
        vec![],
        0,
        None,
        None,
        0,
        [0; 5],
    ))
}

fn dbc_with_houses(houses: &[(u32, u32)]) -> Arc<RwLock<DbcManager>> {
    let mut dbc = DbcManager::new();
    for &(house_id, faction) in houses {
        dbc.auction_house.insert(
            house_id,
            AuctionHouseEntry {
                house_id,
                faction,
                deposit_percent: 5,
                cut_percent: 5,
            },
        );
    }
    Arc::new(RwLock::new(dbc))
}

fn create_test_manager(
    mock_repo: MockAuctionRepositoryTrait,
    dbc: Arc<RwLock<DbcManager>>,
    item_mgr: Arc<ItemManager>,
) -> AuctionHouseManager {
    create_test_manager_with_mail(mock_repo, dbc, item_mgr, MockMailRepositoryTrait::new())
}

fn create_test_manager_with_mail(
    mock_repo: MockAuctionRepositoryTrait,
    dbc: Arc<RwLock<DbcManager>>,
    item_mgr: Arc<ItemManager>,
    mail_repo: MockMailRepositoryTrait,
) -> AuctionHouseManager {
    // Character repo requires a real pool; load_auctions only calls it for seller account lookup.
    // Use a disconnected pool — find_by_guid will fail gracefully and return account 0.
    let pool = Arc::new(
        sqlx::MySqlPool::connect_lazy("mysql://localhost/unused_auction_test")
            .expect("lazy pool"),
    );
    AuctionHouseManager::new(
        Arc::new(mock_repo),
        Arc::new(CharacterRepository::new(pool)),
        Arc::new(mail_repo),
        dbc,
        item_mgr,
    )
}

// ========== PARSER TESTS ==========

#[test]
fn parse_enchantments_empty_string() {
    assert!(parse_enchantments("").is_empty());
}

#[test]
fn parse_enchantments_single_triplet() {
    assert_eq!(parse_enchantments("1 2 3"), vec![(1, 2, 3)]);
}

#[test]
fn parse_enchantments_multiple_triplets() {
    assert_eq!(
        parse_enchantments("1 2 3 4 5 6"),
        vec![(1, 2, 3), (4, 5, 6)]
    );
}

#[test]
fn parse_enchantments_partial_triplet_ignored() {
    assert_eq!(parse_enchantments("1 2"), vec![]);
    assert_eq!(parse_enchantments("1 2 3 4"), vec![(1, 2, 3)]);
}

#[test]
fn parse_enchantments_skips_invalid_tokens() {
    assert_eq!(parse_enchantments("1 x 3"), vec![]);
}

#[test]
fn parse_spell_charges_none() {
    assert_eq!(parse_spell_charges(None), [0; 5]);
}

#[test]
fn parse_spell_charges_partial() {
    assert_eq!(parse_spell_charges(Some("10 -5")), [10, -5, 0, 0, 0]);
}

#[test]
fn parse_spell_charges_caps_at_five() {
    assert_eq!(
        parse_spell_charges(Some("1 2 3 4 5 6 7")),
        [1, 2, 3, 4, 5]
    );
}

// ========== LOAD AUCTION HOUSES TESTS ==========

#[tokio::test]
async fn load_auction_houses_linked_mode_partitions_by_team() {
    let dbc = dbc_with_houses(&[(1, 0), (4, 0), (7, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.load_auction_houses(false, false, 112).unwrap();

    assert!(mgr.has_auction_house_map(1));
    assert!(mgr.has_auction_house_map(4));
    assert!(mgr.has_auction_house_map(7));
}

#[tokio::test]
async fn load_auction_houses_cross_faction_shares_single_object() {
    let dbc = dbc_with_houses(&[(1, 0), (4, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.load_auction_houses(true, false, 112).unwrap();

    assert!(mgr.has_auction_house_map(1));
    assert!(mgr.has_auction_house_map(4));
}

#[tokio::test]
async fn load_auction_houses_unlinked_creates_per_house_maps() {
    let dbc = dbc_with_houses(&[(1, 0), (2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.load_auction_houses(false, true, 100).unwrap();

    assert!(mgr.has_auction_house_map(1));
    assert!(mgr.has_auction_house_map(2));
}

#[tokio::test]
async fn load_auction_houses_unlinked_disabled_on_modern_patch() {
    let dbc = dbc_with_houses(&[(1, 0), (2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    // wow_patch >= 109 falls through to linked mode (still creates maps, but shared by team)
    mgr.load_auction_houses(false, true, 112).unwrap();
    assert!(mgr.has_auction_house_map(1));
}

// ========== MOCK-REPO ASYNC TESTS ==========

fn sample_auction_row(id: u32, house_id: u32, item_guid: u32) -> AuctionRow {
    AuctionRow {
        id,
        house_id,
        item_guid,
        item_id: 25,
        seller_guid: 1,
        buyout_price: 100,
        expire_time: i64::MAX,
        buyer_guid: 0,
        last_bid: 50,
        start_bid: 50,
        deposit: 5,
    }
}

#[tokio::test]
async fn load_auctions_missing_item_deletes_from_db() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mut mock_repo = MockAuctionRepositoryTrait::new();

    mock_repo
        .expect_find_all_for_load()
        .times(1)
        .returning(|| Ok(vec![sample_auction_row(42, 2, 100)]));

    mock_repo
        .expect_delete_auction()
        .with(eq(42))
        .times(1)
        .returning(|_| Ok(()));

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);
    mgr.load_auction_houses(false, false, 112).unwrap();

    mgr.load_auctions().await.unwrap();

    assert_eq!(mgr.auction_count_for_house(2), 0);
}

#[tokio::test]
async fn load_auctions_valid_row_registers_auction() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    item_mgr.add_template(test_item_template(25));

    let mut mock_repo = MockAuctionRepositoryTrait::new();
    mock_repo
        .expect_find_all_for_load()
        .times(1)
        .returning(|| Ok(vec![sample_auction_row(7, 2, 200)]));

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);
    mgr.load_auction_houses(false, false, 112).unwrap();
    mgr.insert_item_for_test(test_item(200, 25));

    mgr.load_auctions().await.unwrap();

    assert_eq!(mgr.auction_count_for_house(2), 1);
}

#[tokio::test]
async fn load_auctions_invalid_house_deletes_and_removes_item() {
    let dbc = dbc_with_houses(&[(7, 0)]); // goblin house only, not house 999
    let item_mgr = Arc::new(ItemManager::new());

    let mut mock_repo = MockAuctionRepositoryTrait::new();
    mock_repo
        .expect_find_all_for_load()
        .times(1)
        .returning(|| Ok(vec![sample_auction_row(99, 999, 300)]));

    mock_repo
        .expect_delete_auction()
        .with(eq(99))
        .times(1)
        .returning(|_| Ok(()));

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);
    mgr.load_auction_houses(false, false, 112).unwrap();
    mgr.insert_item_for_test(test_item(300, 25));

    assert_eq!(mgr.item_count(), 1);

    mgr.load_auctions().await.unwrap();

    assert_eq!(mgr.item_count(), 0);
}

#[tokio::test]
async fn load_auctions_empty_result_succeeds() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());

    let mut mock_repo = MockAuctionRepositoryTrait::new();
    mock_repo
        .expect_find_all_for_load()
        .times(1)
        .returning(|| Ok(vec![]));

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);
    mgr.load_auction_houses(false, false, 112).unwrap();

    mgr.load_auctions().await.unwrap();

    assert_eq!(mgr.auction_count_for_house(2), 0);
}

#[tokio::test]
async fn load_auction_items_query_failure_treated_as_empty() {
    let dbc = dbc_with_houses(&[]);
    let item_mgr = Arc::new(ItemManager::new());

    let mut mock_repo = MockAuctionRepositoryTrait::new();
    mock_repo
        .expect_find_all_items_for_load()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("db unavailable")));

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.load_auction_items().await.unwrap();

    assert_eq!(mgr.item_count(), 0);
}

#[tokio::test]
async fn add_a_item_allows_zero_guid_low() {
    let dbc = dbc_with_houses(&[]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.insert_item_for_test(test_item(0, 25));

    assert_eq!(mgr.item_count(), 1);
}

#[tokio::test]
#[should_panic(expected = "duplicate auction item GUID 401")]
async fn add_a_item_rejects_duplicate_guid_low() {
    let dbc = dbc_with_houses(&[]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();

    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    mgr.insert_item_for_test(test_item(401, 25));
    mgr.insert_item_for_test(test_item(401, 25));
}

// ========== GET AUCTION HOUSE ID FROM FACTION TEMPLATE TESTS ==========

#[test]
fn get_auction_house_id_from_faction_template_known_values() {
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(11), 1);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(12), 1);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(29), 6);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(85), 6);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(55), 2);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(57), 2);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(68), 4);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(71), 4);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(79), 3);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(80), 3);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(104), 5);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(105), 5);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(120), 7);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(474), 7);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(534), 2);
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(855), 7);
}

#[test]
fn get_auction_house_id_from_faction_template_unknown_defaults_to_neutral() {
    assert_eq!(AuctionHouseManager::get_auction_house_id_from_faction_template(9999), 7);
}

// ========== GET AUCTION DEPOSIT TESTS ==========

#[tokio::test]
async fn get_auction_deposit_basic_calculation() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mut template = test_item_template(25);
    template.sell_price = 100;
    item_mgr.add_template(template);

    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let item = test_item(1, 25);
    let house = AuctionHouseEntry {
        house_id: 2,
        faction: 0,
        deposit_percent: 5,
        cut_percent: 5,
    };

    // time = 7200 (1 min auction time unit) => base = 100 * 1 * 1 = 100
    // deposit = 100 * 5 / 100 = 5
    // rate = 1.0, min = 0 => 5
    assert_eq!(mgr.get_auction_deposit(&house, 7200, &item, 0, 1.0), 5);
}

#[tokio::test]
async fn get_auction_deposit_time_below_min_zeros_base() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mut template = test_item_template(25);
    template.sell_price = 100;
    item_mgr.add_template(template);

    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let item = test_item(1, 25);
    let house = AuctionHouseEntry {
        house_id: 2,
        faction: 0,
        deposit_percent: 5,
        cut_percent: 5,
    };

    // time < 7200 => integer division gives 0, base = 0, deposit = 0
    // but min deposit floor = 10 => result = 10
    assert_eq!(mgr.get_auction_deposit(&house, 3600, &item, 10, 1.0), 10);
}

#[tokio::test]
async fn get_auction_deposit_rate_applied() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mut template = test_item_template(25);
    template.sell_price = 100;
    item_mgr.add_template(template);

    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let item = test_item(1, 25);
    let house = AuctionHouseEntry {
        house_id: 2,
        faction: 0,
        deposit_percent: 5,
        cut_percent: 5,
    };

    // base = 100, deposit = 5, rate = 2.0 => 10
    assert_eq!(mgr.get_auction_deposit(&house, 7200, &item, 0, 2.0), 10);
}

#[tokio::test]
async fn get_auction_deposit_stack_count_multiplies() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mut template = test_item_template(25);
    template.sell_price = 100;
    item_mgr.add_template(template);

    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let item = test_item_with_count(1, 25, 5);
    let house = AuctionHouseEntry {
        house_id: 2,
        faction: 0,
        deposit_percent: 5,
        cut_percent: 5,
    };

    // base = 100 * 5 * 1 = 500, deposit = 500 * 5 / 100 = 25
    assert_eq!(mgr.get_auction_deposit(&house, 7200, &item, 0, 1.0), 25);
}

#[tokio::test]
async fn get_auction_deposit_missing_template_returns_zero() {
    let dbc = dbc_with_houses(&[(2, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    // no template added for entry 99

    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let item = test_item(1, 99);
    let house = AuctionHouseEntry {
        house_id: 2,
        faction: 0,
        deposit_percent: 5,
        cut_percent: 5,
    };

    assert_eq!(mgr.get_auction_deposit(&house, 7200, &item, 0, 1.0), 0);
}

// ========== GET CHECKED AUCTION HOUSE FOR AUCTIONEER TESTS ==========

use crate::shared::game::chat::Team;
use crate::world::game::auction::get_checked_auction_house_for_auctioneer;
use crate::world::game::player::Player;

fn test_player(human_guid: u32, race: u8) -> Player {
    Player::new(
        ObjectGuid::new_without_entry(crate::shared::protocol::HighGuid::Player, human_guid),
        "Test".to_string(),
        0,
        0,
        0,
        1,
        race,
        1,
        0,
    )
}

#[tokio::test]
async fn get_checked_auction_house_self_with_access_mode_returns_house() {
    let dbc = dbc_with_houses(&[(1, 0), (6, 0), (7, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let mut player = test_player(42, 1); // human = alliance
    player.auction_access_mode = 1; // neutral

    let house = get_checked_auction_house_for_auctioneer(
        &player,
        player.guid,
        &mgr,
        None,
    );
    assert!(house.is_some());
    assert_eq!(house.unwrap().house_id, 7); // neutral
}

#[tokio::test]
async fn get_checked_auction_house_self_without_access_mode_denies() {
    let dbc = dbc_with_houses(&[(1, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let player = test_player(42, 1); // human = alliance
    // auction_access_mode defaults to 0

    let house = get_checked_auction_house_for_auctioneer(
        &player,
        player.guid,
        &mgr,
        None,
    );
    assert!(house.is_none());
}

#[tokio::test]
async fn get_checked_auction_house_npc_valid_returns_house() {
    let dbc = dbc_with_houses(&[(1, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let player = test_player(42, 1); // human = alliance
    let npc_guid = ObjectGuid::new_without_entry(crate::shared::protocol::HighGuid::Unit, 999);

    // faction template 11 = human = house 1
    let house = get_checked_auction_house_for_auctioneer(
        &player,
        npc_guid,
        &mgr,
        Some(11),
    );
    assert!(house.is_some());
    assert_eq!(house.unwrap().house_id, 1);
}

#[tokio::test]
async fn get_checked_auction_house_npc_invalid_denies() {
    let dbc = dbc_with_houses(&[(1, 0)]);
    let item_mgr = Arc::new(ItemManager::new());
    let mock_repo = MockAuctionRepositoryTrait::new();
    let mgr = create_test_manager(mock_repo, dbc, item_mgr);

    let player = test_player(42, 1);
    let npc_guid = ObjectGuid::new_without_entry(crate::shared::protocol::HighGuid::Unit, 999);

    let house = get_checked_auction_house_for_auctioneer(
        &player,
        npc_guid,
        &mgr,
        None,
    );
    assert!(house.is_none());
}
