use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid as SharedObjectGuid, Opcode, WorldPacket};

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
    fn to_world_packet(&self) -> WorldPacket {
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
}

#[derive(Debug, Clone)]
pub struct SmsgDestroyItem {
    pub item_guid: SharedObjectGuid,
    pub count: u32,
}

impl ToWorldPacket for SmsgDestroyItem {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgDurabilityDamageDeath;

impl ToWorldPacket for SmsgDurabilityDamageDeath {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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

impl ToWorldPacket for SmsgItemQuerySingleResponse {
    fn to_world_packet(&self) -> WorldPacket {
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
        packet.write_u32(self.random_suffix);
        packet.write_u32(self.block);
        packet.write_u32(self.item_set);
        packet.write_u32(self.max_durability);
        packet.write_u32(self.area);
        packet.write_u32(self.map);
        packet.write_u32(self.bag_family);
        packet.write_u32(self.totem_category);
        packet.write_u32(self.socket_color1);
        packet.write_u32(self.socket_color2);
        packet.write_u32(self.socket_color3);
        packet.write_u32(self.socket_bonus);
        packet.write_u32(self.gem_properties);
        packet.write_i32(self.required_disenchant_skill);
        packet.write_f32(self.armor_damage_modifier);
        packet.write_u32(self.duration);
        packet.write_u32(self.item_limit_id);
        packet.write_u32(self.item_limit_category);
        packet.write_u32(self.quality2);

        packet
    }
}

#[derive(Debug, Clone)]
pub struct SmsgBuyBankSlotResult {
    pub result: u8,
}

impl ToWorldPacket for SmsgBuyBankSlotResult {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_READ_ITEM_FAILED);
        packet.write_guid_raw(self.item_guid.raw());
        packet
    }
}
