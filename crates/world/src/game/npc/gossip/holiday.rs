//! Love is in the Air: city NPCs under the Amorous aura greet players with a holiday menu.

/// Amorous, the aura that makes a city NPC take part in the holiday.
pub const SPELL_AMOROUS: u32 = 26869;

const GENDER_MALE: u8 = 0;
const NPC_FLAG_GOSSIP: u32 = 0x1;

use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// The menu an Amorous creature greets with, if it takes part in the holiday.
pub fn amorous_menu(world: &World, creature: ObjectGuid) -> Option<u32> {
    let (entry, display_id, amorous) = world.managers.creature_mgr.with_creature(creature, |c| {
        (c.entry, c.display_id, c.auras.has_aura(SPELL_AMOROUS))
    })?;
    if !amorous {
        return None;
    }
    let gender = world
        .managers
        .creature_mgr
        .get_model_info(display_id)
        .map(|info| info.gender)
        .unwrap_or(GENDER_MALE);
    love_is_in_the_air_menu(entry, gender)
}

/// Amorous came or went: a holiday creature can be talked to while it lasts, and goes
/// back to its own NPC flags afterwards.
pub fn amorous_changed(world: &World, creature: ObjectGuid) {
    let Some(entry) = world.managers.creature_mgr.with_creature(creature, |c| c.entry) else {
        return;
    };
    let template_flags = world
        .managers
        .creature_mgr
        .get_template(entry)
        .map(|t| t.npc_flags)
        .unwrap_or(0);
    let wants_gossip =
        amorous_menu(world, creature).is_some() || template_flags & NPC_FLAG_GOSSIP != 0;
    let changed = world
        .managers
        .creature_mgr
        .with_creature_mut(creature, |c| {
            let flags = if wants_gossip {
                c.npc_flags | NPC_FLAG_GOSSIP
            } else {
                c.npc_flags & !NPC_FLAG_GOSSIP
            };
            (flags != c.npc_flags).then(|| {
                c.npc_flags = flags;
                flags
            })
        })
        .flatten();
    if let Some(flags) = changed {
        crate::game::creature::ai::executor::send_unit_field(
            world,
            creature,
            crate::game::common::update_fields::UNIT_NPC_FLAGS,
            flags,
        );
    }
}

/// The holiday gossip menu a creature uses while Amorous, by entry and (for guards) gender.
pub fn love_is_in_the_air_menu(entry: u32, gender: u8) -> Option<u32> {
    let menu = match entry {
        3518 // Thomas Miller <Baker> (Gender: Male) (Faction: Stormwind)
        => 6954,
        11867 // Woo Ping <Weapon Master> (Gender: Male) (Faction: Stormwind)
        => 6955,
        68 | // Stormwind City Guard (Faction: Stormwind)
        1976 // Stormwind City Patroller (Faction: Stormwind)
        => if gender == GENDER_MALE { 6957 } else { 6958 },
        1402 // Topper McNabb (Gender: Male) (Faction: Stormwind)
        => 6959,
        1303 // Felicia Gump <Herbalism Supplier> (Gender: Female) (Faction: Stormwind)
        => 6960,
        5493 // Arnold Leland <Fishing Trainer> (Gender: Male) (Faction: Stormwind)
        => 6964,
        332 // Master Mathias Shaw <Leader of SI:7> (Gender: Male) (Faction: Stormwind)
        => 6971,
        297 | // Caretaker Folsom (Gender: Male) (Faction: Stormwind)
        338 | // Mazen Mac'Nadir <Academy of Arcane Arts and Sciences> (Gender: Male) (Faction: Stormwind)
        1405 | // Morris Lawry (Gender: Male) (Faction: Stormwind)
        1432 | // Renato Gallina (Gender: Male) (Faction: Stormwind)
        1440 | // Milton Sheaf <Librarian> (Gender: Male) (Faction: Stormwind)
        1646 | // Baros Alexston <City Architect> (Gender: Male) (Faction: Stormwind)
        1719 | // Warden Thelwater (Gender: Male) (Faction: Stormwind)
        2285 | // Count Remington Ridgewell <House of Nobles> (Gender: Male) (Faction: Stormwind)
        2795 | // Lenny "Fingers" McCoy (Gender: Male) (Faction: Stormwind)
        3628 | // Steven Lohan (Gender: Male) (Faction: Stormwind)
        4078 | // Collin Mauren (Gender: Male) (Faction: Stormwind)
        6089 | // Harry Burlguard (Gender: Male) (Faction: Stormwind)
        6175 | // John Turner (Gender: Male) (Faction: Stormwind)
        7917 // Brother Sarno (Gender: Male) (Faction: Stormwind)
        => 6976,
        1305 // Jarel Moor <Bartender> (Gender: Male) (Faction: Stormwind)
        => 6977,
        5498 // Elsharin <Mage Trainer> (Gender: Female) (Faction: Stormwind)
        => 6978,
        5514 // Brooke Stonebraid <Mining Supplier> (Gender: Female) (Faction: Stormwind)
        => 6979,
        11026 // Sprite Jumpsprocket <Journeyman Engineer> (Gender: Female) (Faction: Gnomeregan Exiles)
        => 6980,
        1721 | // Nikova Raskol (Gender: Female) (Faction: Stormwind)
        1750 | // Grand Admiral Jes-Tereth (Gender: Female) (Faction: Stormwind)
        11916 | // Imelda (Gender: Female) (Faction: Stormwind)
        14497 | // Shellene <Orphanage Matron> (Gender: Female) (Faction: Stormwind)
        16002 // Colara Dean (Gender: Female) (Faction: Stormwind)
        => 6981,
        5509 // Kathrum Axehand <Axe Merchant> (Gender: Male) (Faction: Stormwind)
        => 6982,
        957 // Dane Lindgren <Journeyman Blacksmith> (Gender: Male) (Faction: Stormwind)
        => 6983,
        14450 // Orphan Matron Nightingale (Gender: Female) (Faction: Stormwind)
        => 6984,
        5502 // Shylamiir <Herbalism Trainer> (Gender: Female) (Faction: Stormwind)
        => 6985,
        5504 // Sheldras Moontree <Druid Trainer> (Gender: Male) (Faction: Stormwind)
        => 6986,
        3084 // Bluffwatcher (Faction: Thunder Bluff)
        => if gender == GENDER_MALE { 6987 } else { 6988 },
        656 | // Wilder Thistlenettle (Gender: Male) (Faction: Ironforge)
        2912 | // Chief Archaeologist Greywhisker <Explorers' League> (Gender: Male) (Faction: Ironforge)
        3842 | // Brombar Higgleby <The Keymaster> (Gender: Male) (Faction: Ironforge)
        7936 | // Lyon Mountainheart (Gender: Male) (Faction: Ironforge)
        14363 // Thief Catcher Shadowdelve (Gender: Male) (Faction: Ironforge)
        => 6990,
        11028 // Jemma Quikswitch <Journeyman Engineer> (Gender: Female) (Faction: Gnomeregan Exiles)
        => 6991,
        14723 // Mistina Steelshield <Alliance Cloth Quartermaster> (Gender: Female) (Faction: Ironforge)
        => 6993,
        9099 // Sraaz <Pie Vendor> (Gender: Male) (Faction: Gnomeregan Exiles)
        => 6994,
        4256 // Golnir Bouldertoe <Mining Supplier> (Gender: Male) (Faction: Ironforge)
        => 6995,
        8517 // Xiggs Fuselighter (Gender: Male) (Faction: Ironforge)
        => 6996,
        5173 // Alexander Calder <Warlock Trainer> (Gender: Male) (Faction: Ironforge)
        => 6997,
        2489 // Milstaff Stormeye <Portal Trainer> (Gender: Male) (Faction: Ironforge)
        => 6998,
        4262 // Darnassus Sentinel (Gender: Female) (Faction: Darnassus)
        => 6999,
        2796 | // Faelyssa (Gender: Female) (Faction: Darnassus)
        6034 | // Lotherias (Gender: Female) (Faction: Darnassus)
        7313 // Priestess A'moora (Gender: Female) (Faction: Darnassus)
        => 7000,
        7315 // Darnath Bladesinger <Warrior Trainer> (Gender: Male) (Faction: Darnassus)
        => 7001,
        5782 | // Crildor (Gender: Male) (Faction: Darnassus)
        7296 | // Corand (Gender: Male) (Faction: Darnassus)
        11709 // Jareth Wildwoods (Gender: Male) (Faction: Darnassus)
        => 7002,
        4092 // Lariia <Priest Trainer> (Gender: Female) (Faction: Darnassus)
        => 7003,
        10878 // Herald Moonstalker (Gender: Female) (Faction: Darnassus)
        => 7004,
        4214 // Erion Shadewhisper <Rogue Trainer> (Gender: Male) (Faction: Darnassus)
        => 7005,
        3050 | // Veren Tallstrider (Gender: Male) (Faction: Thunder Bluff)
        5599 | // Kon Yelloweyes (Gender: Male) (Faction: Horde Generic)
        14182 | // Bounty Hunter Kolark (Gender: Male) (Faction: Thunder Bluff)
        14440 // Hunter Sagewind (Gender: Male) (Faction: Thunder Bluff)
        => 7008,
        10881 // Bluff Runner Windstrider (Gender: Male) (Faction: Thunder Bluff)
        => 7009,
        8359 // Ahanu <Leather Armor Merchant> (Gender: Male) (Faction: Thunder Bluff)
        => 7010,
        3032 // Beram Skychaser <Shaman Trainer> (Gender: Male) (Faction: Thunder Bluff)
        => 7011,
        3045 // Malakai Cross <Priest Trainer> (Gender: Male) (Faction: Undercity)
        => 7012,
        3048 // Ursyn Ghull <Mage Trainer> (Gender: Female) (Faction: Undercity)
        => 7013,
        12198 // Martin Lindsey <Arathi Basin Battlemaster> (Gender: Male) (Faction: The Defilers)
        => 7014,
        3040 // Urek Thunderhorn <Hunter Trainer> (Gender: Male) (Faction: Thunder Bluff)
        => 7015,
        2802 | // Susan Tillinghast (Gender: Female) (Faction: Undercity)
        5732 // Apothecary Katrina <Royal Apothecary Society> (Gender: Female) (Faction: Undercity)
        => 7016,
        2799 | // Lucian Fenner (Gender: Male) (Faction: Undercity)
        5651 | // Patrick Garrett (Gender: Male) (Faction: Undercity)
        6293 | // Jorah Annison (Gender: Male) (Faction: Undercity)
        6522 | // Andron Gant (Gender: Male) (Faction: Undercity)
        8390 | // Chemist Cuely (Gender: Male) (Faction: Undercity)
        11750 // Ganoosh (Gender: Male) (Faction: Undercity)
        => 7017,
        8403 // Jeremiah Payson <Cockroach Vendor> (Gender: Male) (Faction: Undercity)
        => 7018,
        4595 // Baltus Fowler <Warrior Trainer> (Gender: Male) (Faction: Undercity)
        => 7019,
        4578 // Josephine Lister <Master Shadoweave Tailor> (Gender: Female) (Faction: Undercity)
        => 7020,
        4566 // Kaelystia Hatebringer <Mage Trainer> (Gender: Female) (Faction: Undercity)
        => 7021,
        3296 // Orgrimmar Grunt (Faction: Orgrimmar)
        => if gender == GENDER_MALE { 7022 } else { 7023 },
        8664 // Saern Priderunner (Gender: Female) (Faction: Thunder Bluff)
        => 7024,
        13842 // Frostwolf Ambassador Rokhstrom (Gender: Female) (Faction: Frostwolf Clan)
        => 7025,
        5639 | // Craven Drok (Gender: Male) (Faction: Horde Generic)
        14375 // Scout Stronghand (Gender: Male) (Faction: Orgrimmar)
        => 7026,
        6014 // X'yera <Priest Trainer> (Gender: Male) (Faction: Orgrimmar)
        => 7028,
        8659 // Jes'rimon (Gender: Male) (Faction: Darkspear Trolls)
        => 7029,
        10880 // Warcaller Gorlach (Gender: Male) (Faction: Orgrimmar)
        => 7030,
        13417 // Sagorne Creststrider <Shaman Trainer> (Gender: Male) (Faction: Thunder Bluff)
        => 7031,
        3408 // Zel'mak <Warrior Trainer> (Gender: Male) (Faction: Orgrimmar)
        => 7032,
        5624 // Undercity Guardian (Gender: None) (Faction: Undercity)
        => 7033,
        5595 // Ironforge Guard (Gender: Male) (Faction: Ironforge)
        => 7035,
        7087 // Killian Hagey <Skinning Trainer> (Gender: Male) (Faction: Undercity)
        => 7059,
        4600 | // Geoffrey Hartwell <Weapon Merchant> (Gender: Male) (Faction: Undercity)
        4603 // Nicholas Atwood <Gun Merchant> (Gender: Male) (Faction: Undercity)
        => 7060,
        4554 // Tawny Grisette <Mushroom Vendor> (Gender: Female) (Faction: Undercity)
        => 7063,
        3312 | // Olvia <Meat Vendor> (Gender: Female) (Faction: Orgrimmar)
        3333 | // Shankys <Fishing Supplies> (Gender: Female) (Faction: Orgrimmar)
        3342 | // Shan'ti <Fruit Vendor> (Gender: Female) (Faction: Orgrimmar)
        3346 | // Kithas <Enchanting Supplies> (Gender: Female) (Faction: Orgrimmar)
        3367 | // Felika <General Trade Goods Merchant> (Gender: Female) (Faction: Orgrimmar)
        5188 | // Garyl <Tabard Vendor> (Gender: Female) (Faction: Orgrimmar)
        5816 // Katis <Wand Merchant> (Gender: Female) (Faction: Orgrimmar)
        => 7068,
        8404 // Xan'tish <Snake Vendor> (Gender: Male) (Faction: Orgrimmar)
        => 7069,
        3329 | // Kor'jus <Mushroom Vendor> (Gender: Male) (Faction: Orgrimmar)
        3334 | // Rekkul <Poison Vendor> (Gender: Male) (Faction: Orgrimmar)
        3368 // Borstan <Meat Vendor> (Gender: Male) (Faction: Orgrimmar)
        => 7072,
        3003 | // Fyr Mistrunner <Bread Vendor> (Gender: Female) (Faction: Thunder Bluff)
        8361 // Chepi <Reagent Vendor> (Gender: Female) (Faction: Thunder Bluff)
        => 7074,
        3562 | // Alaindia <Reagent Vendor> (Gender: Female) (Faction: Darnassus)
        4175 | // Vinasia <Cloth Armor Merchant> (Gender: Female) (Faction: Darnassus)
        8665 // Shylenai <Owl Trainer> (Gender: Female) (Faction: Darnassus)
        => 7075,
        4235 | // Turian <Thrown Weapons Merchant> (Gender: Male) (Faction: Darnassus)
        4236 // Cyridan <Leather Armor Merchant> (Gender: Male) (Faction: Darnassus)
        => 7076,
        5112 | // Gwenna Firebrew <Barmaid> (Gender: Female) (Faction: Ironforge)
        5123 | // Bretta Goldfury <Gun Merchant> (Gender: Female) (Faction: Ironforge)
        5155 // Ingrys Stonebrow <Cloth Armor Merchant> (Gender: Female) (Faction: Ironforge)
        => 7077,
        5102 | // Dolman Steelfury <Weapon Merchant> (Gender: Male) (Faction: Ironforge)
        5124 // Sognar Cliffbeard <Meat Vendor> (Gender: Male) (Faction: Ironforge)
        => 7078,
        5158 | // Tilli Thistlefuzz <Enchanting Supplies> (Gender: Female) (Faction: Gnomeregan Exiles)
        7978 // Bimble Longberry <Fruit Vendor> (Gender: Female) (Faction: Gnomeregan Exiles)
        => 7079,
        5169 // Tynnus Venomsprout <Shady Dealer> (Gender: Male) (Faction: Gnomeregan Exiles)
        => 7080,
        1328 | // Elly Langston <Barmaid> (Gender: Female) (Faction: Stormwind)
        12805 // Officer Areyn <Accessories Quartermaster> (Gender: Female) (Faction: Stormwind)
        => 7081,
        _ => return None,
    };
    Some(menu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guards_greet_by_gender() {
        assert_eq!(love_is_in_the_air_menu(68, 0), Some(6957));
        assert_eq!(love_is_in_the_air_menu(68, 1), Some(6958));
        assert_eq!(love_is_in_the_air_menu(3518, 1), Some(6954));
        assert_eq!(love_is_in_the_air_menu(1, 0), None);
    }
}
