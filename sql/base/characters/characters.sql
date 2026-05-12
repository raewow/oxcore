-- MySQL dump
--
-- Table structure for table `characters`
--

DROP TABLE IF EXISTS `characters`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `characters` (
  `guid` int unsigned NOT NULL DEFAULT '0' COMMENT 'Global Unique Identifier',
  `account` int unsigned NOT NULL DEFAULT '0' COMMENT 'Account Identifier',
  `name` varchar(12) NOT NULL DEFAULT '',
  `race` tinyint unsigned NOT NULL DEFAULT '0',
  `class` tinyint unsigned NOT NULL DEFAULT '0',
  `gender` tinyint unsigned NOT NULL DEFAULT '0',
  `skin` tinyint unsigned NOT NULL DEFAULT '0',
  `face` tinyint unsigned NOT NULL DEFAULT '0',
  `hair_style` tinyint unsigned NOT NULL DEFAULT '0',
  `hair_color` tinyint unsigned NOT NULL DEFAULT '0',
  `facial_hair` tinyint unsigned NOT NULL DEFAULT '0',
  `level` tinyint unsigned NOT NULL DEFAULT '0',
  `xp` int unsigned NOT NULL DEFAULT '0',
  `money` int unsigned NOT NULL DEFAULT '0',
  `character_flags` int unsigned NOT NULL DEFAULT '0',
  `zone` int unsigned NOT NULL DEFAULT '0',
  `map` int unsigned NOT NULL DEFAULT '0' COMMENT 'Map Identifier',
  `instance` int unsigned NOT NULL DEFAULT '0',
  `position_x` float NOT NULL DEFAULT '0',
  `position_y` float NOT NULL DEFAULT '0',
  `position_z` float NOT NULL DEFAULT '0',
  `orientation` float NOT NULL DEFAULT '0',
  `transport_guid` bigint unsigned NOT NULL DEFAULT '0',
  `transport_x` float NOT NULL DEFAULT '0',
  `transport_y` float NOT NULL DEFAULT '0',
  `transport_z` float NOT NULL DEFAULT '0',
  `transport_o` float NOT NULL DEFAULT '0',
  `known_taxi_mask` longtext COMMENT 'discovered flight points',
  `current_taxi_path` text COMMENT 'flight destination',
  `online` tinyint unsigned NOT NULL DEFAULT '0',
  `played_time_total` int unsigned NOT NULL DEFAULT '0',
  `played_time_level` int unsigned NOT NULL DEFAULT '0',
  `create_time` bigint unsigned NOT NULL DEFAULT '0',
  `logout_time` bigint unsigned NOT NULL DEFAULT '0',
  `rest_bonus` float NOT NULL DEFAULT '0',
  `reset_talents_multiplier` int unsigned NOT NULL DEFAULT '0',
  `reset_talents_time` bigint unsigned NOT NULL DEFAULT '0',
  `death_expire_time` bigint unsigned NOT NULL DEFAULT '0',
  `stable_slots` tinyint unsigned NOT NULL DEFAULT '0',
  `bank_bag_slots` tinyint unsigned NOT NULL DEFAULT '0',
  `extra_flags` int unsigned NOT NULL DEFAULT '0',
  `honor_rank_points` float NOT NULL DEFAULT '0',
  `honor_highest_rank` int unsigned NOT NULL DEFAULT '0',
  `honor_standing` int unsigned NOT NULL DEFAULT '0',
  `honor_last_week_hk` int unsigned NOT NULL DEFAULT '0',
  `honor_last_week_cp` float NOT NULL DEFAULT '0',
  `honor_stored_hk` int NOT NULL DEFAULT '0',
  `honor_stored_dk` int NOT NULL DEFAULT '0',
  `watched_faction` int NOT NULL DEFAULT '-1',
  `drunk` smallint unsigned NOT NULL DEFAULT '0',
  `health` int unsigned NOT NULL DEFAULT '0',
  `power1` int unsigned NOT NULL DEFAULT '0',
  `power2` int unsigned NOT NULL DEFAULT '0',
  `power3` int unsigned NOT NULL DEFAULT '0',
  `power4` int unsigned NOT NULL DEFAULT '0',
  `power5` int unsigned NOT NULL DEFAULT '0',
  `explored_zones` longtext,
  `equipment_cache` longtext,
  `ammo_id` int unsigned NOT NULL DEFAULT '0',
  `action_bars` tinyint unsigned NOT NULL DEFAULT '0',
  `deleted_account` int unsigned DEFAULT NULL,
  `deleted_name` varchar(12) DEFAULT NULL,
  `deleted_time` bigint DEFAULT NULL,
  `world_phase_mask` int DEFAULT '0',
  PRIMARY KEY (`guid`),
  KEY `idx_account` (`account`),
  KEY `idx_online` (`online`),
  KEY `idx_name` (`name`),
  KEY `idx_instance` (`instance`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Player System';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;