//! Flying a player along taxi paths: talking to flight masters, starting a
//! journey, moving the player through it on the server, landing, and picking
//! a journey back up after a relog.

use std::time::Instant;

use anyhow::Result;
use oxcore_shared::messages::movement::{
    spline_flags, SmsgControlUpdate, SmsgForceMoveUnroot, SmsgMonsterMove, SmsgMoveEnableGravity,
    SmsgMoveUnsetCanFly,
};
use oxcore_shared::messages::taxi::{
    SmsgActivateTaxiReply, SmsgNewTaxiPath, SmsgShowTaxinodes, SmsgTaxinodeStatus, ERR_TAXIOK,
    ERR_TAXINOSUCHPATH, ERR_TAXINOTENOUGHMONEY, ERR_TAXIPLAYERALREADYMOUNTED, ERR_TAXIPLAYERBUSY,
    ERR_TAXIPLAYERSHAPESHIFTED, ERR_TAXITOOFARAWAY, ERR_TAXIUNSPECIFIEDSERVERERROR,
};
use oxcore_shared::protocol::{ObjectGuid, Position, Protocol};

use super::TaxiPathPoint;
use crate::game::common::update_fields::UNIT_FIELD_FLAGS;
use crate::game::common::unit_flags;
use crate::game::inventory::GoldResult;
use crate::World;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};

/// Flight speed in yards per second.
const FLIGHT_SPEED: f32 = 32.0;
/// `UNIT_NPC_FLAG_FLIGHTMASTER`.
const NPC_FLAG_FLIGHTMASTER: u32 = 0x08;
/// How close a player must stand to talk to a flight master.
const INTERACTION_DISTANCE: f32 = 5.0;
/// How far from the starting node a flight may begin.
const MAX_START_DISTANCE: f32 = 31.7;
/// Unit flags held for the whole flight: no client control, on a taxi.
const FLIGHT_UNIT_FLAGS: u32 = unit_flags::DISABLE_MOVE | unit_flags::ON_TAXI;
/// Vanilla spline flags of a taxi flight (Runmode | Flying).
const FLIGHT_SPLINE_FLAGS: u32 = 0x0000_0100 | spline_flags::FLYING;
/// Aura types the flight removes.
const AURA_MOUNTED: u32 = 78;
const AURA_MOD_STEALTH: u32 = 16;
/// Shapeshift forms that may fly (none, warrior stances, shadow, stealth).
const FLIGHT_FORMS: [u8; 6] = [0, 17, 18, 19, 28, 30];

/// A flight in progress: one smooth spline from where the player stood over
/// the points ahead.
#[derive(Debug, Clone)]
pub struct Flight {
    /// The points flown, in order.
    points: Vec<TaxiPathPoint>,
    /// Distance from the start of the spline to each point.
    reached_at: Vec<f32>,
    start: Position,
    started: Instant,
    /// Points already passed.
    passed: usize,
}

impl Flight {
    fn new(start: Position, points: Vec<TaxiPathPoint>) -> Self {
        let mut reached_at = Vec::with_capacity(points.len());
        let mut previous = start;
        let mut total = 0.0;
        for point in &points {
            total += distance(previous, point.position());
            reached_at.push(total);
            previous = point.position();
        }
        Self {
            points,
            reached_at,
            start,
            started: Instant::now(),
            passed: 0,
        }
    }

    fn length(&self) -> f32 {
        self.reached_at.last().copied().unwrap_or(0.0)
    }

    fn duration_ms(&self) -> u32 {
        (self.length() / FLIGHT_SPEED * 1000.0) as u32
    }

    /// Where the flight has got to after `travelled` yards.
    fn position_at(&self, travelled: f32) -> Position {
        let index = self.reached_at.partition_point(|&d| d <= travelled);
        let Some(next) = self.points.get(index) else {
            return self.points.last().map_or(self.start, TaxiPathPoint::position);
        };
        let (from, from_d) = if index == 0 {
            (self.start, 0.0)
        } else {
            (self.points[index - 1].position(), self.reached_at[index - 1])
        };
        let span = self.reached_at[index] - from_d;
        let t = if span > 0.0 {
            ((travelled - from_d) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let to = next.position();
        let facing = (to.y - from.y).atan2(to.x - from.x);
        Position::new(
            from.x + (to.x - from.x) * t,
            from.y + (to.y - from.y) * t,
            from.z + (to.z - from.z) * t,
            facing.rem_euclid(std::f32::consts::TAU),
        )
    }
}

fn distance(a: Position, b: Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn send_reply(world: &World, player_guid: ObjectGuid, reply: u32) {
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, SmsgActivateTaxiReply { reply });
}

/// The node at a flight master, if the player may talk to them.
fn flight_master_node(world: &World, player_guid: ObjectGuid, npc: ObjectGuid) -> Option<u32> {
    let (map_id, instance_id, position, team, alive) =
        world.managers.player_mgr.with_player(player_guid, |player| {
            (
                player.map_id,
                player.instance_id,
                player.movement.position,
                player.get_team(),
                player.is_alive(),
            )
        })?;
    let npc_position = world
        .managers
        .creature_mgr
        .with_creature(npc, |creature| {
            (creature.npc_flags & NPC_FLAG_FLIGHTMASTER != 0
                && creature.is_alive()
                && creature.map_id == map_id
                && creature.instance_id == instance_id)
                .then_some(creature.position)
        })
        .flatten()?;
    if !alive || distance(position, npc_position) > INTERACTION_DISTANCE {
        return None;
    }
    world
        .managers
        .taxi_mgr
        .network()
        .nearest_node(map_id, npc_position, team)
}

/// CMSG_TAXINODE_STATUS_QUERY: whether the player knows the flight point at
/// this flight master (shown on their head).
pub fn send_taxi_status(world: &World, player_guid: ObjectGuid, npc: ObjectGuid) {
    let Some((map_id, team)) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| (player.map_id, player.get_team()))
    else {
        return;
    };
    let Some(npc_position) = world
        .managers
        .creature_mgr
        .with_creature(npc, |creature| {
            (creature.map_id == map_id).then_some(creature.position)
        })
        .flatten()
    else {
        return;
    };
    let Some(node) = world
        .managers
        .taxi_mgr
        .network()
        .nearest_node(map_id, npc_position, team)
    else {
        return;
    };
    let is_known = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.taxi.knows(node))
        .unwrap_or(false);
    world.managers.broadcast_mgr.send_msg_to_player(
        player_guid,
        SmsgTaxinodeStatus {
            creature_guid: npc,
            is_known,
        },
    );
}

/// CMSG_TAXIQUERYAVAILABLENODES (and the flight master gossip option): learn
/// the flight point if it is new, otherwise open the flight map.
pub fn talk_to_flight_master(world: &World, player_guid: ObjectGuid, npc: ObjectGuid) {
    let Some(node) = flight_master_node(world, player_guid, npc) else {
        return;
    };
    if learn_node(world, player_guid, npc, node) {
        return;
    }
    send_taxi_menu(world, player_guid, npc, node);
}

/// Greeting a flight master teaches its flight point. True when the point was new, or
/// when the flight master has none (so there is no map to show either).
pub fn learn_flight_master_node(world: &World, player_guid: ObjectGuid, npc: ObjectGuid) -> bool {
    match flight_master_node(world, player_guid, npc) {
        Some(node) => learn_node(world, player_guid, npc, node),
        None => true,
    }
}

/// Teach the player `node` and tell them, if they did not know it.
fn learn_node(world: &World, player_guid: ObjectGuid, npc: ObjectGuid, node: u32) -> bool {
    let learned = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| player.taxi.learn(node))
        .unwrap_or(false);
    if learned {
        let broadcast = &world.managers.broadcast_mgr;
        broadcast.send_msg_to_player(player_guid, SmsgNewTaxiPath {});
        broadcast.send_msg_to_player(
            player_guid,
            SmsgTaxinodeStatus {
                creature_guid: npc,
                is_known: true,
            },
        );
        save(world, player_guid);
    }
    learned
}

fn send_taxi_menu(world: &World, player_guid: ObjectGuid, npc: ObjectGuid, node: u32) {
    let Some(taxi_mask) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.taxi.known)
    else {
        return;
    };
    world.managers.broadcast_mgr.send_msg_to_player(
        player_guid,
        SmsgShowTaxinodes {
            creature_guid: npc,
            current_node: node,
            taxi_mask,
        },
    );
}

/// CMSG_ACTIVATETAXI from 1.14, which names only the destination: route it
/// over the nodes the player knows.
pub async fn activate_to(
    world: &World,
    player_guid: ObjectGuid,
    npc: ObjectGuid,
    destination: u32,
) -> Result<bool> {
    let Some(source) = flight_master_node(world, player_guid, npc) else {
        return Ok(false);
    };
    let Some(known) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.taxi.known)
    else {
        return Ok(false);
    };
    let Some(route) = world.managers.taxi_mgr.network().route(source, destination, &known) else {
        send_reply(world, player_guid, ERR_TAXINOSUCHPATH);
        return Ok(false);
    };
    activate(world, player_guid, &route, Some(npc), 0).await
}

/// Start a flight over the scripted path a spell names.
pub async fn activate_path(
    world: &World,
    player_guid: ObjectGuid,
    path: u32,
    spell_id: u32,
) -> Result<bool> {
    let Some((from, to)) = world.managers.taxi_mgr.network().path_ends(path) else {
        return Ok(false);
    };
    activate(world, player_guid, &[from, to], None, spell_id).await
}

/// Start a journey through `nodes`, from a flight master (`npc`) or a spell
/// or script (`npc` None). Returns whether the player took off.
pub async fn activate(
    world: &World,
    player_guid: ObjectGuid,
    nodes: &[u32],
    npc: Option<ObjectGuid>,
    spell_id: u32,
) -> Result<bool> {
    if nodes.len() < 2 {
        return Ok(false);
    }
    // A flight master journey needs the player at that flight master.
    if let Some(npc) = npc {
        if flight_master_node(world, player_guid, npc).is_none() {
            return Ok(false);
        }
    }
    let taxi = world.managers.taxi_mgr.network();
    let logging_out = world
        .session_mgr
        .get_session_by_player(player_guid)
        .is_some_and(|session| session.logout_time_remaining(u32::MAX).is_some());
    let in_combat = world
        .systems
        .combat
        .is_in_combat(player_guid, &world.managers.player_mgr);
    if logging_out || in_combat {
        send_reply(world, player_guid, ERR_TAXIPLAYERBUSY);
        return Ok(false);
    }

    struct Rider {
        unit_flags: u32,
        knows_all: bool,
        mounted: bool,
        form: u8,
        casting: bool,
        map_id: u32,
        position: Position,
        team: oxcore_shared::game::chat::Team,
    }
    let Some(rider) = world.managers.player_mgr.with_player(player_guid, |player| Rider {
        unit_flags: player.unit_flags,
        knows_all: nodes.iter().all(|&node| player.taxi.knows(node)),
        mounted: player.mount_display_id != 0,
        form: player.shapeshift_form,
        casting: player.spells.is_non_melee_spell_casted(false, false, false),
        map_id: player.map_id,
        position: player.movement.position,
        team: player.get_team(),
    }) else {
        return Ok(false);
    };
    if rider.unit_flags & unit_flags::DISABLE_MOVE != 0 {
        return Ok(false);
    }
    // Spell flights may cross nodes the player has never visited.
    if npc.is_some() && !rider.knows_all {
        tracing::warn!("Player {:?} tried to fly through unknown nodes", player_guid);
        return Ok(false);
    }
    let shapeshifted = !FLIGHT_FORMS.contains(&rider.form);
    if npc.is_some() {
        let refusal = if rider.mounted {
            Some(ERR_TAXIPLAYERALREADYMOUNTED)
        } else if shapeshifted {
            Some(ERR_TAXIPLAYERSHAPESHIFTED)
        } else if rider.casting {
            Some(ERR_TAXIPLAYERBUSY)
        } else {
            None
        };
        if let Some(reply) = refusal {
            send_reply(world, player_guid, reply);
            return Ok(false);
        }
    } else {
        world
            .systems
            .auras
            .remove_spells_causing_aura(player_guid, AURA_MOUNTED, world)
            .await?;
    }

    let source = nodes[0];
    let Some(node) = taxi.node(source) else {
        send_reply(world, player_guid, ERR_TAXINOSUCHPATH);
        return Ok(false);
    };
    let node_placed = node.position.x != 0.0 || node.position.y != 0.0 || node.position.z != 0.0;
    if node_placed {
        if node.map_id != rider.map_id || distance(node.position, rider.position) > MAX_START_DISTANCE
        {
            send_reply(world, player_guid, ERR_TAXITOOFARAWAY);
            return Ok(false);
        }
    } else if npc.is_some() {
        send_reply(world, player_guid, ERR_TAXIUNSPECIFIEDSERVERERROR);
        return Ok(false);
    }

    let discount = npc.map_or(1.0, |npc| reputation_discount(world, player_guid, npc));
    let mut first_cost = 0u32;
    let mut total_cost = 0u32;
    for (i, hop) in nodes.windows(2).enumerate() {
        let Some((_, price)) = taxi.path(hop[0], hop[1]) else {
            tracing::error!("No taxi path from node {} to {}", hop[0], hop[1]);
            return Ok(false);
        };
        let cost = (price as f32 * discount + 0.5) as u32;
        if i == 0 {
            first_cost = cost;
        }
        total_cost += cost;
    }
    let Some(points) = taxi.journey_points(nodes) else {
        send_reply(world, player_guid, ERR_TAXIUNSPECIFIEDSERVERERROR);
        return Ok(false);
    };
    let mount_display = mount_display(world, source, rider.team, npc.is_none());
    if mount_display == 0 && spell_id == 0 {
        send_reply(world, player_guid, ERR_TAXIUNSPECIFIEDSERVERERROR);
        return Ok(false);
    }
    let money = world.systems.inventory.get_money(player_guid).unwrap_or(0);
    if money < total_cost {
        send_reply(world, player_guid, ERR_TAXINOTENOUGHMONEY);
        return Ok(false);
    }

    // Take off.
    crate::game::player::teleport::stop_combat(world, player_guid, false);
    if first_cost > 0
        && !matches!(
            world.systems.inventory.remove_gold(player_guid, first_cost),
            GoldResult::Success { .. }
        )
    {
        send_reply(world, player_guid, ERR_TAXINOTENOUGHMONEY);
        return Ok(false);
    }
    world
        .systems
        .auras
        .remove_spells_causing_aura(player_guid, AURA_MOD_STEALTH, world)
        .await?;
    // The pet waits out the flight and comes back on landing.
    crate::game::player::teleport::put_pet_away(world, player_guid);
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.taxi.destinations = nodes.iter().copied().collect();
            player.taxi.discount = discount;
        });
    send_reply(world, player_guid, ERR_TAXIOK);
    start_flight(world, player_guid, mount_display, points);
    save(world, player_guid);
    Ok(true)
}

/// The mount a flight from `node` shows, as a display id.
fn mount_display(world: &World, node: u32, team: oxcore_shared::game::chat::Team, any_side: bool) -> u32 {
    world
        .managers
        .taxi_mgr
        .network()
        .mount_creature(node, team, any_side)
        .and_then(|entry| world.managers.creature_mgr.get_template(entry))
        .map_or(0, |template| template.get_display_id())
}

/// The flight master's reputation discount: 10% off from Honored.
fn reputation_discount(world: &World, player_guid: ObjectGuid, npc: ObjectGuid) -> f32 {
    use oxcore_shared::game::reputation::ReputationRank;

    let Some(template) = world
        .managers
        .creature_mgr
        .with_creature(npc, |creature| creature.faction)
    else {
        return 1.0;
    };
    let Some(faction) = world
        .dbc
        .read()
        .get_faction_template(template)
        .map(|entry| entry.faction)
    else {
        return 1.0;
    };
    let mut discount = match world
        .systems
        .reputation
        .get_rank(player_guid, faction, world)
    {
        Some(rank) if rank >= ReputationRank::Honored => 0.9,
        _ => 1.0,
    };
    // Capital cities' flight masters also reward PvP rank: 5% from Corporal
    // (visual rank 2), 10% from Master Sergeant (4).
    const CAPITAL_FACTIONS: [u32; 10] = [72, 47, 54, 69, 730, 68, 76, 81, 530, 729];
    if CAPITAL_FACTIONS.contains(&faction) {
        let visual_rank = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.honor.rank.visual_rank)
            .unwrap_or(0);
        if visual_rank >= 2 {
            discount -= 0.05;
        }
        if visual_rank >= 4 {
            discount -= 0.05;
        }
    }
    discount
}

/// Mount up, take control away and send the flight spline over `points`.
fn start_flight(world: &World, player_guid: ObjectGuid, mount_display: u32, points: Vec<TaxiPathPoint>) {
    let Some(start) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.movement.position)
    else {
        return;
    };
    // The spline covers the points on the current map; a path that crosses
    // maps lands at the edge and continues after a teleport.
    let map_id = points.first().map(|point| point.map_id).unwrap_or(0);
    let on_map: Vec<TaxiPathPoint> = points
        .iter()
        .take_while(|point| point.map_id == map_id)
        .copied()
        .collect();
    let flight = Flight::new(start, on_map);
    let duration = flight.duration_ms();
    let waypoints: Vec<Position> = flight.points.iter().map(TaxiPathPoint::position).collect();

    let unit_flags = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            if mount_display != 0 {
                player.mount_display_id = mount_display;
            }
            player.unit_flags |= FLIGHT_UNIT_FLAGS;
            player.taxi.flight = Some(flight);
            player.unit_flags
        })
        .unwrap_or(FLIGHT_UNIT_FLAGS);
    crate::game::player::teleport::put_pet_away(world, player_guid);

    let broadcast = &world.managers.broadcast_mgr;
    let mut fields = ValuesUpdateBlock::new(player_guid, ObjectType::Player)
        .set_field(UNIT_FIELD_FLAGS, unit_flags);
    if mount_display != 0 {
        fields = fields.set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_MOUNTDISPLAYID,
            mount_display,
        );
    }
    broadcast.broadcast_msg_nearby(
        player_guid,
        &SmsgUpdateObject::new().add_block(UpdateBlockData::Values(fields)),
        true,
    );

    // 1.14 also needs to be told it no longer steers.
    if is_modern(world, player_guid) {
        for _ in 0..2 {
            broadcast.send_msg_to_player(player_guid, SmsgMonsterMove::new_stop(player_guid, start));
            broadcast.send_msg_to_player(
                player_guid,
                SmsgControlUpdate {
                    guid: player_guid,
                    has_control: false,
                },
            );
        }
    }

    let spline = SmsgMonsterMove {
        guid: player_guid,
        position: start,
        spline_id: rand::random(),
        move_type: 0,
        facing_target: None,
        facing_angle: None,
        spline_flags: FLIGHT_SPLINE_FLAGS,
        duration,
        waypoints,
    };
    broadcast.broadcast_msg_nearby(player_guid, &spline, true);
}

fn is_modern(world: &World, player_guid: ObjectGuid) -> bool {
    world
        .session_mgr
        .get_session_by_player(player_guid)
        .is_some_and(|session| session.protocol() == Protocol::Modern)
}

/// Move every flying player along their flight. Called from the world tick.
pub async fn update(world: &World) -> Result<()> {
    let flyers: Vec<ObjectGuid> = world
        .managers
        .player_mgr
        .collect_online_guids()
        .into_iter()
        .filter(|&guid| {
            world
                .managers
                .player_mgr
                .with_player(guid, |player| player.taxi.is_flying())
                .unwrap_or(false)
        })
        .collect();
    for player_guid in flyers {
        advance(world, player_guid).await?;
    }
    Ok(())
}

async fn advance(world: &World, player_guid: ObjectGuid) -> Result<()> {
    // Where the flight is now, which points were passed, whether it is over.
    let Some((old, new, hops_started, finished, map_id, instance_id)) = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let flight = player.taxi.flight.as_mut()?;
            let travelled = flight.started.elapsed().as_secs_f32() * FLIGHT_SPEED;
            let finished = travelled >= flight.length();
            let mut hops_started = 0;
            while flight.passed < flight.points.len()
                && flight.reached_at[flight.passed] <= travelled
            {
                let here = flight.points[flight.passed].path;
                let next = flight.points.get(flight.passed + 1).map(|point| point.path);
                flight.passed += 1;
                if next.is_some_and(|next| next != here) {
                    hops_started += 1;
                }
            }
            let new = flight.position_at(travelled);
            let old = player.movement.position;
            player.movement.position = new;
            Some((old, new, hops_started, finished, player.map_id, player.instance_id))
        })
        .flatten()
    else {
        return Ok(());
    };

    let map = world
        .managers
        .map_mgr
        .get_or_create_map(map_id, instance_id);
    map.relocate_player(player_guid, old, new);
    world
        .systems
        .player
        .visibility()
        .check_cell_crossing(player_guid, old, new, world);

    // Each new hop of a multi-hop journey is paid for as it begins.
    for _ in 0..hops_started {
        let cost = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.taxi.destinations.pop_front();
                let taxi = &player.taxi;
                if taxi.destinations.len() < 2 {
                    return 0;
                }
                world
                    .managers
                    .taxi_mgr
                    .network()
                    .path(taxi.destinations[0], taxi.destinations[1])
                    .map_or(0, |(_, price)| (price as f32 * taxi.discount + 0.5) as u32)
            })
            .unwrap_or(0);
        if cost > 0 {
            // Short of money mid-air still lands the player where they paid for.
            let _ = world.systems.inventory.remove_gold(player_guid, cost);
        }
        save(world, player_guid);
    }

    if finished {
        land(world, player_guid).await?;
    }
    Ok(())
}

/// The spline is over: dismount, give control back, then fly the next hop,
/// cross to the next map, or land at the destination.
async fn land(world: &World, player_guid: ObjectGuid) -> Result<()> {
    end_flight_state(world, player_guid);

    let taxi = world.managers.taxi_mgr.network();
    let Some((destinations, map_id, team)) =
        world.managers.player_mgr.with_player(player_guid, |player| {
            (player.taxi.destinations.clone(), player.map_id, player.get_team())
        })
    else {
        return Ok(());
    };
    let Some(&destination) = destinations.get(1) else {
        clear_journey(world, player_guid);
        return Ok(());
    };
    let Some(dest_node) = taxi.node(destination).cloned() else {
        clear_journey(world, player_guid);
        return Ok(());
    };

    // The path continues on another map: carry on from its first point there.
    if dest_node.map_id != map_id {
        let next = taxi
            .path(destinations[0], destination)
            .and_then(|(path, _)| taxi.path_points(path))
            .and_then(|points| points.iter().find(|point| point.map_id != map_id).copied());
        if let Some(point) = next {
            crate::game::player::teleport::teleport_to(
                world,
                player_guid,
                point.map_id,
                point.position(),
                0,
            )
            .await?;
        }
        clear_journey(world, player_guid);
        return Ok(());
    }

    // More hops queued (a resumed journey): fly the next one from its second point.
    if destinations.len() > 2 {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.taxi.destinations.pop_front();
            });
        let (source, next) = (destinations[1], destinations[2]);
        let points = taxi
            .path(source, next)
            .and_then(|(path, _)| taxi.path_points(path))
            .map(|points| points.iter().skip(1).copied().collect::<Vec<_>>());
        let mount = mount_display(world, source, team, false);
        if let Some(points) = points.filter(|points| !points.is_empty() && mount != 0) {
            start_flight(world, player_guid, mount, points);
            save(world, player_guid);
            return Ok(());
        }
    }

    clear_journey(world, player_guid);
    // Land exactly on the node, as the client may have finished early.
    let mut landing = dest_node.position;
    landing.o = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.movement.position.o)
        .unwrap_or(0.0);
    crate::game::player::teleport::teleport_to(world, player_guid, dest_node.map_id, landing, 0)
        .await?;
    Ok(())
}

/// Dismount and hand control back after a spline, keeping the journey.
fn end_flight_state(world: &World, player_guid: ObjectGuid) {
    let Some((unit_flags, was_mounted)) = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.taxi.flight.take()?;
            player.unit_flags &= !FLIGHT_UNIT_FLAGS;
            let was_mounted = player.mount_display_id != 0;
            player.mount_display_id = 0;
            Some((player.unit_flags, was_mounted))
        })
        .flatten()
    else {
        return;
    };

    let broadcast = &world.managers.broadcast_mgr;
    let mut fields = ValuesUpdateBlock::new(player_guid, ObjectType::Player)
        .set_field(UNIT_FIELD_FLAGS, unit_flags);
    if was_mounted {
        fields = fields.set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_MOUNTDISPLAYID,
            0u32,
        );
    }
    broadcast.broadcast_msg_nearby(
        player_guid,
        &SmsgUpdateObject::new().add_block(UpdateBlockData::Values(fields)),
        true,
    );
    if is_modern(world, player_guid) {
        broadcast.send_msg_to_player(
            player_guid,
            SmsgControlUpdate {
                guid: player_guid,
                has_control: true,
            },
        );
        broadcast.send_msg_to_player(player_guid, SmsgMoveEnableGravity { guid: player_guid });
        broadcast.send_msg_to_player(player_guid, SmsgMoveUnsetCanFly { guid: player_guid });
        broadcast.send_msg_to_player(player_guid, SmsgForceMoveUnroot { guid: player_guid });
    }
    crate::game::player::teleport::resummon_pet(world, player_guid);
}

fn clear_journey(world: &World, player_guid: ObjectGuid) {
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.taxi.destinations.clear();
            player.taxi.discount = 1.0;
        });
    save(world, player_guid);
}

/// Stop any flight at once (the player is being teleported elsewhere).
pub fn interrupt(world: &World, player_guid: ObjectGuid) {
    let flying = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.taxi.is_flying() || !player.taxi.destinations.is_empty())
        .unwrap_or(false);
    if flying {
        end_flight_state(world, player_guid);
        clear_journey(world, player_guid);
    }
}

/// Set up a logging-in player's taxi state from their saved mask and journey.
pub fn load_for_login(
    world: &World,
    taxi: &mut super::PlayerTaxi,
    race: u8,
    stored_mask: Option<&str>,
    stored_journey: Option<&str>,
) {
    let network = world.managers.taxi_mgr.network();
    let network = network.network_mask();
    match stored_mask.filter(|mask| !mask.trim().is_empty()) {
        Some(mask) => taxi.load_mask(mask, network),
        None => {
            // A new character starts with their race's capital and hubs.
            let start = world
                .dbc
                .read()
                .chr_races
                .lookup(u32::from(race))
                .map_or(0, |entry| entry.starting_taxi_mask);
            taxi.known.0[0] = start;
        }
    }
    taxi.discount = 1.0;
    let nodes: Vec<u32> = stored_journey
        .unwrap_or("")
        .split_whitespace()
        .filter_map(|node| node.parse().ok())
        .collect();
    let valid = nodes.len() >= 2
        && nodes
            .windows(2)
            .all(|hop| world.managers.taxi_mgr.network().path(hop[0], hop[1]).is_some());
    if valid {
        taxi.destinations = nodes.into_iter().collect();
    }
}

/// After login: pick a saved journey back up from the point of the current
/// hop's path nearest the player.
pub fn resume(world: &World, player_guid: ObjectGuid) {
    let taxi = world.managers.taxi_mgr.network();
    let Some((source, destination, position, map_id, team)) =
        world.managers.player_mgr.with_player(player_guid, |player| {
            let d = &player.taxi.destinations;
            (d.len() >= 2).then(|| {
                (
                    d[0],
                    d[1],
                    player.movement.position,
                    player.map_id,
                    player.get_team(),
                )
            })
        })
        .flatten()
    else {
        return;
    };
    let mount = mount_display(world, source, team, true);
    let Some(points) = taxi
        .path(source, destination)
        .and_then(|(path, _)| taxi.path_points(path))
    else {
        clear_journey(world, player_guid);
        return;
    };
    // The first point past which the player already is: between it and the
    // one before, the two distances add up to less than the segment.
    let mut start = 0;
    for i in 1..points.len() {
        if points[i].map_id != map_id {
            continue;
        }
        let segment = distance(points[i - 1].position(), points[i].position());
        let to_prev = distance(points[i - 1].position(), position);
        let to_next = distance(points[i].position(), position);
        if to_prev + to_next < segment * 1.05 {
            start = i;
            break;
        }
    }
    start_flight(world, player_guid, mount, points[start..].to_vec());
}

/// Persist the known nodes and the hop in progress.
pub fn save(world: &World, player_guid: ObjectGuid) {
    let Some((mask, journey)) = world.managers.player_mgr.with_player(player_guid, |player| {
        (player.taxi.mask_to_string(), player.taxi.journey_to_string())
    }) else {
        return;
    };
    let pool = world.databases.character.clone();
    let guid = i64::from(player_guid.counter());
    tokio::spawn(async move {
        let repository =
            oxcore_db::database::characters::PgCharacterRepository::new(std::sync::Arc::new(pool));
        if let Err(e) = repository.save_taxi(guid, &mask, &journey).await {
            tracing::warn!("Failed to save taxi state of character {}: {}", guid, e);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32) -> TaxiPathPoint {
        TaxiPathPoint {
            path: 1,
            map_id: 0,
            x,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn flight_moves_at_flight_speed_along_its_points() {
        let flight = Flight::new(Position::new(0.0, 0.0, 0.0, 0.0), vec![point(32.0), point(96.0)]);
        assert_eq!(flight.length(), 96.0);
        assert_eq!(flight.duration_ms(), 3000);
        let half = flight.position_at(16.0);
        assert!((half.x - 16.0).abs() < 1e-3);
        let later = flight.position_at(64.0);
        assert!((later.x - 64.0).abs() < 1e-3);
        let end = flight.position_at(500.0);
        assert!((end.x - 96.0).abs() < 1e-3);
    }
}
