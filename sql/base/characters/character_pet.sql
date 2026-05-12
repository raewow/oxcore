-- MySQL dump
--
-- Table structure for table `character_pet`
--

DROP TABLE IF EXISTS `character_pet`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `character_pet` (
  `id` int unsigned NOT NULL DEFAULT '0',
  `entry` int unsigned NOT NULL DEFAULT '0',
  `owner_guid` int unsigned NOT NULL DEFAULT '0',
  `display_id` int unsigned DEFAULT '0',
  `created_by_spell` int unsigned NOT NULL DEFAULT '0',
  `pet_type` tinyint unsigned NOT NULL DEFAULT '0',
  `level` int unsigned NOT NULL DEFAULT '1',
  `xp` int unsigned NOT NULL DEFAULT '0',
  `react_state` tinyint unsigned NOT NULL DEFAULT '0',
  `loyalty_points` int NOT NULL DEFAULT '0',
  `loyalty` int unsigned NOT NULL DEFAULT '0',
  `training_points` int NOT NULL DEFAULT '0',
  `name` varchar(100) DEFAULT 'Pet',
  `renamed` tinyint unsigned NOT NULL DEFAULT '0',
  `slot` int unsigned NOT NULL DEFAULT '0',
  `current_health` int unsigned NOT NULL DEFAULT '1',
  `current_mana` int unsigned NOT NULL DEFAULT '0',
  `current_happiness` int unsigned NOT NULL DEFAULT '0',
  `save_time` bigint unsigned NOT NULL DEFAULT '0',
  `reset_talents_cost` int unsigned NOT NULL DEFAULT '0',
  `reset_talents_time` bigint unsigned NOT NULL DEFAULT '0',
  `action_bar_data` longtext,
  `teach_spell_data` longtext,
  PRIMARY KEY (`id`),
  KEY `idx_owner` (`owner_guid`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Pet System';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;