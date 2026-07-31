use crate::messages::update::{SmsgUpdateObject, DEFAULT_REALM_ID};
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid as SharedObjectGuid, Opcode, WorldPacket};

/// Serialize a 1.14 item instance for build 42597.
///
/// Every 1.14 body that names an item embeds this rather than a bare item id. A 1.12 item has
/// neither bonus lists nor modifications, so both sub-structures are written empty — but they are
/// still written: the client reads the presence bit and the 6-bit modification count
/// unconditionally, so skipping them shifts every field after the instance.
///
/// The two counts live in separate bit runs, each closed by its own flush, because the presence bit
/// and the count are not adjacent on the wire.
pub(crate) fn write_modern_item_instance(
    writer: &mut BitWriter,
    item_id: u32,
    random_properties_seed: u32,
    random_properties_id: u32,
) {
    writer.write_u32(item_id);
    writer.write_u32(random_properties_seed);
    writer.write_u32(random_properties_id);
    writer.write_bit(false); // HasItemBonus -- 1.12 has no item bonus lists
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
}

/// 1.14 DisplayType replaces vanilla's three independent u32 flags.
const DISPLAY_TYPE_HIDDEN: u32 = 0;
const DISPLAY_TYPE_RECEIVED: u32 = 1;
const DISPLAY_TYPE_LOOT: u32 = 3;

#[derive(Debug, Clone)]
pub struct SmsgItemPushResult {
    pub player_guid: SharedObjectGuid,
    pub received: u8,
    pub created: u8,
    pub show_in_chat: u8,
    pub bagslot: u8,
    pub item_entry: u32,
    pub suffix_factor: u32,
    pub random_property_id: u32,
    pub count: u32,
}

impl ToWorldPacket for SmsgItemPushResult {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_PUSH_RESULT);
        packet.write_guid_raw(self.player_guid.raw());
        packet.write_u32(self.received as u32);
        packet.write_u32(self.created as u32);
        packet.write_u32(self.show_in_chat as u32);
        packet.write_u8(self.bagslot);
        packet.write_u32(self.item_entry);
        packet.write_u32(self.suffix_factor);
        packet.write_u32(self.random_property_id);
        packet.write_u32(self.count);
        packet
    }

    /// `ItemPushResult::Write` for build 42597.
    ///
    /// Vanilla's three u32 booleans collapse into a 3-bit `DisplayText` selector plus a `Pushed`
    /// bit, so they cannot be copied across one by one. The mapping is not arbitrary: an item that
    /// came from an NPC and was not conjured is a "received" toast anchored to the pushing NPC,
    /// anything with the chat flag clear is silently hidden, and everything else renders as loot.
    /// Get it wrong and quest rewards either announce themselves twice or not at all.
    ///
    /// Two fields have no source in the 1.12 body and are sent as documented blanks:
    ///
    /// * `SlotInBag` is `-1`. Vanilla carries a slot-within-bag alongside the bag slot, but this
    ///   struct never captured it, and `-1` is the value 1.14 reads as "no particular slot" — a
    ///   real-looking `0` would make the client animate the wrong bag square.
    /// * `QuantityInInventory` repeats `Quantity`. 1.12 sends only the per-pickup delta, so the
    ///   over-head quest toast reads `n/N` with `n` counting this pickup alone rather than the
    ///   running total.
    ///
    /// `ItemGUID` is empty for the same reason: the 1.12 body identifies the item by entry only, and
    /// the client falls back to the entry when the GUID is absent.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // PlayerGUID

        writer.write_u8(self.bagslot); // Slot
        writer.write_i32(-1); // SlotInBag -- see above
        writer.write_i32(0); // QuestLogItemID -- 1.12 never overrides the quest-credit item
        writer.write_u32(self.count); // Quantity
        writer.write_u32(self.count); // QuantityInInventory -- see above
        writer.write_i32(0); // DungeonEncounterID
        writer.write_i32(0); // BattlePetSpeciesID
        writer.write_i32(0); // BattlePetBreedID
        writer.write_u32(0); // BattlePetBreedQuality
        writer.write_i32(0); // BattlePetLevel
        writer.write_packed_guid_128(0, 0); // ItemGUID -- see above

        let from_npc = self.received != 0;
        let created = self.created != 0;
        let display_type = if from_npc && !created {
            DISPLAY_TYPE_RECEIVED
        } else if self.show_in_chat == 0 {
            DISPLAY_TYPE_HIDDEN
        } else {
            DISPLAY_TYPE_LOOT
        };

        writer.write_bit(display_type == DISPLAY_TYPE_RECEIVED); // Pushed
        writer.write_bit(created); // Created
        writer.write_bits(display_type, 3); // DisplayText
        writer.write_bit(false); // IsBonusRoll -- no bonus rolls in Classic Era
        writer.write_bit(false); // IsEncounterLoot
        writer.flush_bits();

        write_modern_item_instance(
            &mut writer,
            self.item_entry,
            self.suffix_factor,
            self.random_property_id,
        );

        Some(writer.finish(Opcode::SMSG_ITEM_PUSH_RESULT))
    }
}

#[derive(Debug, Clone)]
pub struct SmsgDestroyItem {
    pub item_guid: SharedObjectGuid,
    pub count: u32,
}

impl ToWorldPacket for SmsgDestroyItem {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }

    /// 1.14 has no `SMSG_DESTROY_OBJECT`; destruction is a list inside `SMSG_UPDATE_OBJECT`.
    fn to_modern(&self) -> Option<WorldPacket> {
        SmsgUpdateObject::new().destroy(self.item_guid).to_modern()
    }
}

#[derive(Debug, Clone)]
pub struct SmsgOpenContainer {
    pub item_guid: SharedObjectGuid,
}

/// No `to_modern`: the 1.14 opcode exists but its body layout could not be established.
///
/// Unlike the sibling item messages, no authoritative field list for the 1.14 body was available,
/// and a bare "it is probably just the widened GUID" guess is not safe here -- the 1.14 item
/// messages that *are* documented all append fields the vanilla body has no trace of, so a
/// GUID-only body is as likely to be short as it is to be right. This needs a real layout before it
/// can be written.
impl ToWorldPacket for SmsgOpenContainer {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_OPEN_CONTAINER);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgDurabilityDamageDeath;

impl ToWorldPacket for SmsgDurabilityDamageDeath {
    /// Vanilla's empty packet gains a percentage in 1.14, because the client now prints the number
    /// in the on-death message instead of hardcoding it.
    ///
    /// The value is not carried on the wire in 1.12 and this struct is a unit type, so it is
    /// restated here rather than derived: it must match the rate the death handler actually applies
    /// to equipment, which is a flat 10% of maximum durability per item. If that rate is ever made
    /// configurable, this literal becomes a lie the client repeats to the player.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(10); // Percent -- see above
        Some(writer.finish(Opcode::SMSG_DURABILITY_DAMAGE_DEATH))
    }

    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_DURABILITY_DAMAGE_DEATH)
    }
}

#[derive(Debug, Clone)]
pub struct SmsgItemEnchantTimeUpdate {
    pub item_guid: SharedObjectGuid,
    pub slot: u32,
    pub duration: u32,
    pub caster_guid: Option<SharedObjectGuid>,
}

impl ToWorldPacket for SmsgItemEnchantTimeUpdate {
    /// **The slot and the duration swap places.** 1.14 writes `item, duration, slot, owner`; vanilla
    /// writes `item, slot, duration, owner`. Both are u32, so sending them in vanilla order does not
    /// fail to parse -- it tells the client that a temporary enchant in slot *(duration)* has
    /// *(slot)* milliseconds left, which typically reads as an enchant that expires instantly.
    ///
    /// The trailing GUID is the item's **owner**, not the enchanter, despite this struct naming it
    /// `caster_guid`; that is what the vanilla body carries there too. When it is absent the empty
    /// GUID is still written, because 1.14 reads it unconditionally.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.item_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.duration); // DurationLeft -- ahead of the slot, see above
        writer.write_u32(self.slot);
        let (high, low) = self
            .caster_guid
            .unwrap_or_default()
            .to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // OwnerGuid
        Some(writer.finish(Opcode::SMSG_ITEM_ENCHANT_TIME_UPDATE))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_ENCHANT_TIME_UPDATE);
        packet.write_guid_raw(self.item_guid.raw());
        packet.write_u32(self.slot);
        packet.write_u32(self.duration);
        if let Some(guid) = self.caster_guid {
            packet.write_guid_raw(guid.raw());
        } else {
            packet.write_u8(0);
        }
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgItemNameQueryResponse<'a> {
    pub item_id: u32,
    pub name: &'a str,
}

/// No `to_modern`: 1.14 never asks the question, so there is nothing to answer.
///
/// The 1.14 opcode table has no item-name query response at all. A 1.14 client resolves item names
/// from its own local item data and never sends the request, so this message is unreachable on a
/// modern session rather than merely unported.
impl ToWorldPacket for SmsgItemNameQueryResponse<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_NAME_QUERY_RESPONSE);
        packet.write_u32(self.item_id);
        packet.write_cstring(self.name);
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgItemQuerySingleResponse {
    pub entry: u32,
    pub class: u32,
    pub subclass: u32,
    pub name_0: String,
    pub name_1: String,
    pub name_2: String,
    pub name_3: String,
    pub display_id: u32,
    pub quality: u32,
    pub flags: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u32,
    pub allowable_class: i32,
    pub allowable_race: i32,
    pub item_level: u32,
    pub required_level: u32,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_spell: u32,
    pub required_honor_rank: u32,
    pub required_city_rank: u32,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub max_count: u32,
    pub stackable: u32,
    pub container_slots: u32,
    pub stat_type1: i32,
    pub stat_value1: i32,
    pub stat_type2: i32,
    pub stat_value2: i32,
    pub stat_type3: i32,
    pub stat_value3: i32,
    pub stat_type4: i32,
    pub stat_value4: i32,
    pub stat_type5: i32,
    pub stat_value5: i32,
    pub stat_type6: i32,
    pub stat_value6: i32,
    pub stat_type7: i32,
    pub stat_value7: i32,
    pub stat_type8: i32,
    pub stat_value8: i32,
    pub stat_type9: i32,
    pub stat_value9: i32,
    pub stat_type10: i32,
    pub stat_value10: i32,
    pub damage_min1: f32,
    pub damage_max1: f32,
    pub damage_type1: u32,
    pub damage_min2: f32,
    pub damage_max2: f32,
    pub damage_type2: u32,
    pub damage_min3: f32,
    pub damage_max3: f32,
    pub damage_type3: u32,
    pub damage_min4: f32,
    pub damage_max4: f32,
    pub damage_type4: u32,
    pub damage_min5: f32,
    pub damage_max5: f32,
    pub damage_type5: u32,
    pub armor: u32,
    pub holy_res: u32,
    pub fire_res: u32,
    pub nature_res: u32,
    pub frost_res: u32,
    pub shadow_res: u32,
    pub arcane_res: u32,
    pub delay: u32,
    pub ammo_type: u32,
    pub ranged_mod_range: f32,
    pub spell_id1: u32,
    pub spell_trigger1: u32,
    pub spell_charges1: i32,
    pub spell_cooldown1: u32,
    pub spell_category1: u32,
    pub spell_category_cooldown1: u32,
    pub spell_id2: u32,
    pub spell_trigger2: u32,
    pub spell_charges2: i32,
    pub spell_cooldown2: u32,
    pub spell_category2: u32,
    pub spell_category_cooldown2: u32,
    pub spell_id3: u32,
    pub spell_trigger3: u32,
    pub spell_charges3: i32,
    pub spell_cooldown3: u32,
    pub spell_category3: u32,
    pub spell_category_cooldown3: u32,
    pub spell_id4: u32,
    pub spell_trigger4: u32,
    pub spell_charges4: i32,
    pub spell_cooldown4: u32,
    pub spell_category4: u32,
    pub spell_category_cooldown4: u32,
    pub spell_id5: u32,
    pub spell_trigger5: u32,
    pub spell_charges5: i32,
    pub spell_cooldown5: u32,
    pub spell_category5: u32,
    pub spell_category_cooldown5: u32,
    pub bonding: u32,
    pub description: String,
    pub page_text_id: u32,
    pub language_id: u32,
    pub page_material: u32,
    pub start_quest: u32,
    pub lock_id: u32,
    pub material: u32,
    pub sheath: u32,
    pub random_property: i32,
    pub random_suffix: u32,
    pub block: u32,
    pub item_set: u32,
    pub max_durability: u32,
    pub area: u32,
    pub map: u32,
    pub bag_family: u32,
    pub totem_category: u32,
    pub socket_color1: u32,
    pub socket_color2: u32,
    pub socket_color3: u32,
    pub socket_bonus: u32,
    pub gem_properties: u32,
    pub required_disenchant_skill: i32,
    pub armor_damage_modifier: f32,
    pub duration: u32,
    pub item_limit_id: u32,
    pub item_limit_category: u32,
    pub quality2: u32,
}

/// No `to_modern`: same reason as [`SmsgItemNameQueryResponse`] -- 1.14 has no item query.
///
/// The whole item template is client-side data in 1.14; the opcode pair was removed and the client
/// never sends the request. Every 1.14 message that names an item embeds only an item id and lets
/// the client look the rest up, which is why `write_modern_item_instance` is so much smaller than
/// this struct.
impl ToWorldPacket for SmsgItemQuerySingleResponse {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_QUERY_SINGLE_RESPONSE);
        packet.write_u32(self.entry);
        packet.write_u32(self.class);
        packet.write_u32(self.subclass);
        packet.write_string(&self.name_0);
        if self.name_1.is_empty() {
            packet.write_u8(0);
        } else {
            packet.write_string(&self.name_1);
        }
        if self.name_2.is_empty() {
            packet.write_u8(0);
        } else {
            packet.write_string(&self.name_2);
        }
        if self.name_3.is_empty() {
            packet.write_u8(0);
        } else {
            packet.write_string(&self.name_3);
        }
        packet.write_u32(self.display_id);
        packet.write_u32(self.quality);
        packet.write_u32(self.flags);
        packet.write_u32(self.buy_price);
        packet.write_u32(self.sell_price);
        packet.write_u32(self.inventory_type);
        packet.write_i32(self.allowable_class);
        packet.write_i32(self.allowable_race);
        packet.write_u32(self.item_level);
        packet.write_u32(self.required_level);
        packet.write_u32(self.required_skill);
        packet.write_u32(self.required_skill_rank);
        packet.write_u32(self.required_spell);
        packet.write_u32(self.required_honor_rank);
        packet.write_u32(self.required_city_rank);
        packet.write_u32(self.required_reputation_faction);
        packet.write_u32(self.required_reputation_rank);
        packet.write_u32(self.max_count);
        packet.write_u32(self.stackable);
        packet.write_u32(self.container_slots);

        for i in 1..=10 {
            let stat_type = match i {
                1 => self.stat_type1,
                2 => self.stat_type2,
                3 => self.stat_type3,
                4 => self.stat_type4,
                5 => self.stat_type5,
                6 => self.stat_type6,
                7 => self.stat_type7,
                8 => self.stat_type8,
                9 => self.stat_type9,
                10 => self.stat_type10,
                _ => 0,
            };
            let stat_value = match i {
                1 => self.stat_value1,
                2 => self.stat_value2,
                3 => self.stat_value3,
                4 => self.stat_value4,
                5 => self.stat_value5,
                6 => self.stat_value6,
                7 => self.stat_value7,
                8 => self.stat_value8,
                9 => self.stat_value9,
                10 => self.stat_value10,
                _ => 0,
            };
            packet.write_i32(stat_type);
            packet.write_i32(stat_value);
        }

        for i in 1..=5 {
            let (dmg_min, dmg_max, dmg_type) = match i {
                1 => (self.damage_min1, self.damage_max1, self.damage_type1),
                2 => (self.damage_min2, self.damage_max2, self.damage_type2),
                3 => (self.damage_min3, self.damage_max3, self.damage_type3),
                4 => (self.damage_min4, self.damage_max4, self.damage_type4),
                5 => (self.damage_min5, self.damage_max5, self.damage_type5),
                _ => (0.0, 0.0, 0),
            };
            packet.write_f32(dmg_min);
            packet.write_f32(dmg_max);
            packet.write_u32(dmg_type);
        }

        packet.write_u32(self.armor);
        packet.write_u32(self.holy_res);
        packet.write_u32(self.fire_res);
        packet.write_u32(self.nature_res);
        packet.write_u32(self.frost_res);
        packet.write_u32(self.shadow_res);
        packet.write_u32(self.arcane_res);
        packet.write_u32(self.delay);
        packet.write_u32(self.ammo_type);
        packet.write_f32(self.ranged_mod_range);

        for i in 1..=5 {
            let (
                spell_id,
                spell_trigger,
                spell_charges,
                spell_cooldown,
                spell_category,
                spell_category_cooldown,
            ) = match i {
                1 => (
                    self.spell_id1,
                    self.spell_trigger1,
                    self.spell_charges1,
                    self.spell_cooldown1,
                    self.spell_category1,
                    self.spell_category_cooldown1,
                ),
                2 => (
                    self.spell_id2,
                    self.spell_trigger2,
                    self.spell_charges2,
                    self.spell_cooldown2,
                    self.spell_category2,
                    self.spell_category_cooldown2,
                ),
                3 => (
                    self.spell_id3,
                    self.spell_trigger3,
                    self.spell_charges3,
                    self.spell_cooldown3,
                    self.spell_category3,
                    self.spell_category_cooldown3,
                ),
                4 => (
                    self.spell_id4,
                    self.spell_trigger4,
                    self.spell_charges4,
                    self.spell_cooldown4,
                    self.spell_category4,
                    self.spell_category_cooldown4,
                ),
                5 => (
                    self.spell_id5,
                    self.spell_trigger5,
                    self.spell_charges5,
                    self.spell_cooldown5,
                    self.spell_category5,
                    self.spell_category_cooldown5,
                ),
                _ => (0, 0, 0, 0, 0, 0),
            };
            packet.write_u32(spell_id);
            packet.write_u32(spell_trigger);
            packet.write_i32(spell_charges);
            packet.write_u32(spell_cooldown);
            packet.write_u32(spell_category);
            packet.write_u32(spell_category_cooldown);
        }

        packet.write_u32(self.bonding);
        packet.write_string(&self.description);
        packet.write_u32(self.page_text_id);
        packet.write_u32(self.language_id);
        packet.write_u32(self.page_material);
        packet.write_u32(self.start_quest);
        packet.write_u32(self.lock_id);
        packet.write_u32(self.material);
        packet.write_u32(self.sheath);
        packet.write_i32(self.random_property);
        packet.write_u32(self.block);
        packet.write_u32(self.item_set);
        packet.write_u32(self.max_durability);
        packet.write_u32(self.area);
        packet.write_u32(self.map);
        packet.write_u32(self.bag_family);

        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgBuyBankSlotResult {
    pub result: u8,
}

/// No `to_modern`: 1.14 removed the reply and drives the bank UI from object fields instead.
///
/// The 1.14 opcode table keeps `CMSG_BUY_BANK_SLOT` but has no result opcode for it. The client
/// learns the purchase happened from the bank-slot count on its own player object, so the correct
/// modern behaviour is to send that field update -- not to invent a body for a message the client
/// has no handler for.
impl ToWorldPacket for SmsgBuyBankSlotResult {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BUY_BANK_SLOT_RESULT);
        packet.write_u8(self.result);
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgReadItemOk {
    pub item_guid: SharedObjectGuid,
}

impl ToWorldPacket for SmsgReadItemOk {
    /// The same single GUID, widened to 128 bits and packed.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.item_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        Some(writer.finish(Opcode::SMSG_READ_ITEM_OK))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_READ_ITEM_OK);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgReadItemFailed {
    pub item_guid: SharedObjectGuid,
}

impl ToWorldPacket for SmsgReadItemFailed {
    /// 1.14 appends a retry delay and a 2-bit failure subcode to the GUID vanilla sends alone.
    ///
    /// Both are new fields with no vanilla source. The delay is 0 because vanilla's failure is
    /// permanent rather than "try again in N seconds", and the subcode is the generic
    /// cannot-read case -- vanilla has exactly one failure and does not say which of the 1.14
    /// subcases it is.
    ///
    /// The subcode occupies 2 bits of a byte the client reads whole, so the run is closed
    /// explicitly: the body is one byte longer than the fields suggest.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// The 1.14 subcode meaning "this item cannot be read", as opposed to the
        /// still-being-generated and language-barrier cases vanilla has no equivalent for.
        const SUBCODE_CANT_READ: u32 = 2;

        let mut writer = BitWriter::new();
        let (high, low) = self.item_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(0); // Delay -- see above
        writer.write_bits(SUBCODE_CANT_READ, 2);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_READ_ITEM_FAILED))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_READ_ITEM_FAILED);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }
}
