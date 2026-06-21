use crate::game::player::player::Player;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

const WORLD_BOSS_LEVEL_DIFF: u32 = 3;
const SPELL_SCHOOL_MASK_NORMAL: u32 = 0x01;

fn get_unit_level(guid: ObjectGuid, world: &World) -> Option<u32> {
    if guid.is_player() {
        return world
            .managers
            .player_mgr
            .with_player(guid, |player: &Player| player.level as u32);
    }

    if guid.is_creature() || guid.is_pet() {
        return world
            .managers
            .creature_mgr
            .with_creature(guid, |creature| creature.level as u32);
    }

    None
}

/// Faithful `SpellCaster::GetLevelForTarget` port.
pub fn get_level_for_target(
    source_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    world: &World,
) -> u32 {
    if let Some((creature_level, creature_entry)) = world
        .managers
        .creature_mgr
        .with_creature(source_guid, |creature| (creature.level as u32, creature.entry))
    {
        if world
            .managers
            .creature_mgr
            .get_template(creature_entry)
            .is_some_and(|template| template.rank == 3)
        {
            if let Some(target_guid) = target_guid.filter(|guid| guid.is_unit()) {
                if let Some(target_level) = get_unit_level(target_guid, world) {
                    return target_level
                        .saturating_add(WORLD_BOSS_LEVEL_DIFF)
                        .clamp(1, 255);
                }
            }
        }

        return creature_level;
    }

    if let Some(level) = world
        .managers
        .player_mgr
        .with_player(source_guid, |player: &Player| player.level as u32)
    {
        return level;
    }

    if let Some(level) = world
        .managers
        .gameobject_mgr
        .with_gameobject(source_guid, |go| go.level)
    {
        if level != 0 {
            return level;
        }
    }

    if let Some(target_guid) = target_guid.filter(|guid| guid.is_unit()) {
        if let Some(level) = get_unit_level(target_guid, world) {
            return level;
        }
    }

    world.config.max_player_level.clamp(1, 255)
}

/// Faithful `SpellCaster::GetMeleeDamageSchoolMask` port.
pub fn get_melee_damage_school_mask(
    _source_guid: ObjectGuid,
    _attack_type: u8,
    _world: &World,
) -> u32 {
    SPELL_SCHOOL_MASK_NORMAL
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::creature::Creature;
    use crate::game::creature::manager::CreatureTemplate;
    use crate::game::gameobject::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::player::player::Player;
    use crate::World;
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::{ObjectGuid, Position};
    use sqlx::mysql::MySqlPoolOptions;
    use std::sync::Arc;
    use std::path::PathBuf;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn creature_template(entry: u32, rank: u8) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature{entry}"),
            subname: None,
            min_level: 50,
            max_level: 50,
            faction: 1,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn creature(guid: ObjectGuid, entry: u32, rank: u8) -> Creature {
        Creature::new(
            guid,
            entry,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            1,
            0,
            &creature_template(entry, rank),
            1,
            None,
        )
    }

    fn gameobject(guid: ObjectGuid, level: u32) -> GameObject {
        let template = GameObjectTemplate {
            entry: guid.entry(),
            go_type: 0,
            display_id: 1,
            name: format!("GO{}", guid.entry()),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data: [0; 24],
        };
        let mut go = GameObject::new(
            guid,
            template.entry,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            1,
            &template,
            [0.0, 0.0, 0.0, 1.0],
            0,
            0,
        );
        go.level = level;
        go
    }

    #[tokio::test]
    async fn player_source_returns_player_level() {
        let world = test_world();
        let source = ObjectGuid::new_player(1);
        world
            .managers
            .player_mgr
            .add_player(Player::new(source, "P".into(), 1, 0, 0, 37, 1, 1, 0), 1);

        assert_eq!(get_level_for_target(source, None, &world), 37);
    }

    #[tokio::test]
    async fn creature_source_returns_creature_level() {
        let world = test_world();
        let source = ObjectGuid::new_creature(100, 1);
        world
            .managers
            .creature_mgr
            .add_template(creature_template(100, 0));
        world
            .managers
            .creature_mgr
            .add_creature_for_test(creature(source, 100, 0));

        assert_eq!(get_level_for_target(source, None, &world), 50);
    }

    #[tokio::test]
    async fn world_boss_source_scales_against_unit_targets() {
        let world = test_world();
        let source = ObjectGuid::new_creature(200, 1);
        let target = ObjectGuid::new_player(2);

        world
            .managers
            .creature_mgr
            .add_template(creature_template(200, 3));
        world
            .managers
            .creature_mgr
            .add_creature_for_test(creature(source, 200, 3));
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 60, 1, 1, 0), 2);

        assert_eq!(get_level_for_target(source, Some(target), &world), 63);
    }

    #[tokio::test]
    async fn gameobject_source_uses_own_level_then_target_then_max_level() {
        let world = test_world();
        let source = ObjectGuid::new_gameobject(300, 1);
        let target = ObjectGuid::new_player(2);

        world.managers.gameobject_mgr.add_gameobject_for_test(gameobject(source, 12));
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 45, 1, 1, 0), 2);

        assert_eq!(get_level_for_target(source, Some(target), &world), 12);

        world.managers.gameobject_mgr.add_gameobject_for_test(gameobject(source, 0));
        assert_eq!(get_level_for_target(source, Some(target), &world), 45);

        assert_eq!(get_level_for_target(source, None, &world), 60);
    }

    #[tokio::test]
    async fn unknown_source_uses_target_unit_or_max_level() {
        let world = test_world();
        let target = ObjectGuid::new_player(2);
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 29, 1, 1, 0), 2);

        assert_eq!(get_level_for_target(ObjectGuid::new_gameobject(999, 1), Some(target), &world), 29);
        assert_eq!(get_level_for_target(ObjectGuid::new_gameobject(999, 1), None, &world), 60);
    }

    #[tokio::test]
    async fn melee_damage_school_mask_is_always_physical() {
        let world = test_world();
        let player = ObjectGuid::new_player(3);

        assert_eq!(get_melee_damage_school_mask(player, 0, &world), SPELL_SCHOOL_MASK_NORMAL);
        assert_eq!(get_melee_damage_school_mask(ObjectGuid::new_creature(1, 1), 0, &world), SPELL_SCHOOL_MASK_NORMAL);
    }
}
