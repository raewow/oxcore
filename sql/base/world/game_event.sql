-- MySQL dump
--
-- Table structure for table `game_event`
-- Table data for table `game_event`
--

DROP TABLE IF EXISTS `game_event`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `game_event` (
  `entry` mediumint unsigned NOT NULL COMMENT 'Entry of the game event',
  `start_time` timestamp NOT NULL DEFAULT '0000-00-00 00:00:00' COMMENT 'Absolute start date, the event will never start before',
  `end_time` timestamp NOT NULL DEFAULT '0000-00-00 00:00:00' COMMENT 'Absolute end date, the event will never start afler',
  `occurence` bigint unsigned NOT NULL DEFAULT '5184000' COMMENT 'Delay in minutes between occurences of the event',
  `length` bigint unsigned NOT NULL DEFAULT '2592000' COMMENT 'Length in minutes of the event',
  `holiday` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'Client side holiday id',
  `description` varchar(255) DEFAULT NULL COMMENT 'Description of the event displayed in console',
  `hardcoded` tinyint NOT NULL DEFAULT '0',
  `disabled` tinyint NOT NULL DEFAULT '0',
  `patch_min` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Minimum content patch to enable this event',
  `patch_max` tinyint unsigned NOT NULL DEFAULT '10' COMMENT 'Maximum content patch to enable this event',
  PRIMARY KEY (`entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `game_event`
--

LOCK TABLES `game_event` WRITE;
/*!40000 ALTER TABLE `game_event` DISABLE KEYS */;
INSERT INTO `game_event` (`entry`, `start_time`, `end_time`, `occurence`, `length`, `holiday`, `description`, `hardcoded`, `disabled`, `patch_min`, `patch_max`) VALUES
(1, '2020-06-21 20:00:00', '2037-12-31 23:59:59', 525600, 20160, 341, 'Midsummer Fire Festival', 0, 0, 9, 10),
(2, '2020-12-16 23:00:00', '2037-12-31 23:59:59', 525600, 25980, 141, 'Feast of Winter Veil', 0, 0, 0, 10),
(4, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 2592000, 374, 'Darkmoon Faire (Elwynn)', 1, 0, 4, 10),
(5, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 2592000, 375, 'Darkmoon Faire (Mulgore)', 1, 0, 4, 10),
(6, '2017-01-01 04:00:00', '2037-01-01 00:00:00', 525600, 10, 0, 'Fireworks', 1, 0, 0, 10),
(7, '2020-01-24 22:00:00', '2037-12-31 23:59:59', 525600, 20160, 327, 'Lunar Festival', 0, 0, 7, 10),
(8, '2021-02-11 22:00:00', '2037-02-11 23:00:00', 525600, 7200, 335, 'Love is in the Air', 0, 0, 7, 10),
(10, '2020-05-01 20:00:00', '2037-12-31 23:59:59', 525600, 10080, 201, 'Children\'s Week', 0, 0, 2, 10),
(11, '2020-09-29 22:00:00', '2037-12-31 23:59:59', 525600, 10080, 321, 'Harvest Festival', 0, 0, 5, 10),
(12, '2020-10-18 20:00:00', '2037-12-31 23:59:59', 525600, 20160, 324, 'Hallow\'s End', 0, 0, 6, 10),
(22, '2020-08-03 00:00:00', '2037-12-31 04:00:00', 1, 2592000, 0, 'AQ War Effort (NPC & GO \'Initial\')', 1, 0, 7, 7),
(17, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion', 1, 1, 9, 10),
(13, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion', 1, 0, 2, 10),
(14, '2017-01-07 09:00:00', '2037-01-01 00:00:00', 10080, 1920, 0, 'Stranglethorn Fishing Extravaganza: Announce', 0, 0, 5, 10),
(16, '2020-12-31 23:00:00', '2037-12-31 23:59:59', 180, 120, 0, 'Gurubashi Arena Booty Run', 0, 0, 2, 10),
(15, '2017-01-08 14:00:00', '2037-01-01 00:00:00', 10080, 120, 301, 'Stranglethorn Fishing Extravaganza: Tournament', 0, 0, 5, 10),
(19, '2020-05-08 00:00:01', '2037-12-31 23:59:59', 30240, 6240, 284, 'Call to Arms, Warsong Gulch', 0, 0, 5, 10),
(20, '2020-05-15 00:00:01', '2037-12-31 23:59:59', 30240, 6240, 285, 'Call to Arms, Arathi Basin', 0, 0, 5, 10),
(23, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 2592000, 374, 'Darkmoon Faire Building (Elwynn)', 1, 0, 4, 10),
(25, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 1, 0, 'Call to Arms', 0, 0, 5, 10),
(27, '2020-01-02 21:00:00', '2037-12-31 09:00:00', 1440, 720, 0, 'Night', 0, 0, 0, 10),
(29, '2020-06-10 00:00:01', '2037-12-31 23:59:59', 80640, 20160, 0, 'Edge of Madness, Gri\'lek', 0, 0, 5, 10),
(30, '2020-06-23 00:00:01', '2037-12-31 23:59:59', 80640, 20160, 0, 'Edge of Madness, Hazza\'rah', 0, 0, 5, 10),
(31, '2020-05-12 00:00:01', '2037-12-31 23:59:59', 80640, 20160, 0, 'Edge of Madness, Renataki', 0, 0, 5, 10),
(32, '2020-05-26 00:00:01', '2037-12-31 23:59:59', 80640, 20160, 0, 'Edge of Madness, Wushoolay', 0, 0, 5, 10),
(28, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 2880, 0, 'Noblegarden', 0, 0, 0, 10),
(33, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 5184000, 2592000, 0, 'Arena Tournament', 0, 0, 0, 10),
(34, '2020-12-31 06:00:00', '2037-01-01 07:00:05', 525600, 1445, 0, 'New Year\'s Eve', 0, 0, 0, 10),
(35, '2010-03-24 11:00:00', '2037-01-01 05:00:00', 1260, 420, 0, 'The Maul: Mushgog', 0, 0, 1, 10),
(36, '2010-03-24 18:00:00', '2037-01-01 05:00:00', 1260, 420, 0, 'The Maul: Skarr the Unbreakable', 0, 0, 1, 10),
(37, '2010-03-25 01:00:00', '2037-01-01 05:00:00', 1260, 420, 0, 'The Maul: Razza', 0, 0, 1, 10),
(40, '2017-01-08 12:00:00', '2037-01-01 00:00:00', 10080, 300, 0, 'Stranglethorn Fishing Extravaganza: The Crew', 0, 0, 5, 10),
(47, '2017-01-01 06:00:00', '2037-12-31 01:00:00', 1440, 1080, 0, 'Fishing: Sunscale Cycle', 0, 0, 0, 10),
(45, '2020-03-20 10:00:00', '2037-01-01 05:00:00', 525600, 262800, 0, 'Fishing: summer period (for Raw Summer Bass)', 0, 0, 0, 10),
(46, '2020-09-23 09:00:00', '2037-01-01 05:00:00', 525600, 262800, 0, 'Fishing: winter period (for Winter Squid)', 0, 0, 0, 10),
(53, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Effort (NPC Reput Named \'Officer\')', 1, 0, 7, 7),
(52, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Effort (NPC Reput \'Only War Effort\')', 1, 0, 7, 7),
(54, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus (NPC & GO) DAY 1', 1, 0, 7, 7),
(55, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus (NPC & GO) DAY 2', 1, 0, 7, 7),
(56, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus (NPC & GO) DAY 3', 1, 0, 7, 7),
(57, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus (NPC & GO) DAY 4', 1, 0, 7, 7),
(58, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus (NPC & GO) DAY 5', 1, 0, 7, 7),
(59, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Effort (Cenarion Hold attack)', 1, 0, 7, 7),
(60, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Troop Silithus 3 (NPC & GO)', 1, 0, 7, 7),
(61, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War Final Battle (NPC)', 1, 0, 7, 7),
(62, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War - Secrets of the Colossus Ashi', 0, 0, 7, 7),
(63, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War - Secrets of the Colossus Regal', 0, 0, 7, 7),
(64, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War - Secrets of the Colossus Zora', 0, 0, 7, 7),
(65, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2592000, 0, 'AQ War - Spawn of Crystals', 1, 0, 7, 7),
(18, '2020-05-01 00:00:01', '2037-12-31 23:59:59', 30240, 6240, 283, 'Call to Arms: Alterac Valley', 0, 0, 5, 10),
(24, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 525600, 2592000, 375, 'Darkmoon Faire Building (Mulgore)', 1, 0, 4, 10),
(68, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Fire (Un\'goro Crater)', 1, 0, 2, 10),
(103, '2020-09-08 04:00:00', '2037-12-31 04:00:00', 525600, 33720, 0, 'Event PVP Silithus', 0, 0, 10, 10),
(129, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Phase 2 - Invasion Stormwind', 0, 0, 9, 10),
(130, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Phase 2 - Invasion Undercity', 0, 0, 9, 10),
(81, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Boss in instance activation', 0, 0, 9, 10),
(90, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Winterspring', 1, 1, 9, 10),
(49, '2020-01-02 21:00:00', '2037-12-31 06:30:00', 1440, 480, 0, 'Pyrewood Village Night', 0, 0, 0, 10),
(152, '2022-01-01 00:19:48', '2037-03-31 03:19:48', 120, 35, 0, 'Southshore Assassins', 0, 0, 0, 10),
(66, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Dragons of Nightmare Spawn', 1, 0, 6, 10),
(69, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Air (Silithus)', 1, 0, 2, 10),
(70, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Earth (Azshara)', 1, 0, 2, 10),
(71, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Water (Winterspring)', 1, 0, 2, 10),
(72, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Baron Charr (Un\'goro Crater)', 1, 0, 2, 10),
(73, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: The Windreaver (Silithus)', 1, 0, 2, 10),
(74, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Avalanchion (Azshara)', 1, 0, 2, 10),
(75, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Elemental Invasion: Princess Tempestria (Winterspring)', 1, 0, 2, 10),
(48, '2017-01-01 18:00:00', '2037-12-31 13:00:00', 1440, 1080, 0, 'Fishing: Nightfin Cycle', 0, 0, 0, 10),
(39, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 60, 10, 0, 'Post Firework Toasting Goblets', 1, 0, 6, 10),
(78, '2010-01-01 01:00:00', '2037-01-01 00:00:00', 60, 1, 0, 'Hourly Bells', 0, 0, 0, 10),
(83, '2005-01-01 00:00:00', '2037-01-01 00:00:00', 1, 2592000, 0, 'AQ Gate', 0, 0, 0, 7),
(84, '2005-01-01 00:00:00', '2037-01-01 00:00:00', 1, 2, 0, 'AQ War Effort (master event)', 1, 0, 7, 10),
(85, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'AQ War Effort (gong)', 1, 0, 7, 10),
(86, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 2, 0, 'AQ War Effort (post-event)', 1, 0, 7, 10),
(21, '2020-12-25 00:00:00', '2037-01-03 00:59:59', 525600, 12960, 0, 'Feast of Winter Veil: Gifts', 0, 0, 0, 10),
(110, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Darnassus', 0, 1, 7, 10),
(38, '2020-02-08 06:00:00', '2037-02-09 07:00:00', 525600, 1445, 0, 'Chinese New Year', 0, 0, 0, 10),
(41, '2020-06-04 00:00:00', '2037-06-05 02:00:00', 525600, 1440, 0, 'July 4th Fireworks Spectacular (US Only)', 0, 0, 0, 10),
(42, '2020-09-30 00:00:00', '2037-09-30 02:00:00', 525600, 1440, 0, 'September 30th Peon Day (EU Only)', 0, 0, 0, 10),
(43, 'NaN-NaN-NaN NaN:NaN:NaN', 'NaN-NaN-NaN NaN:NaN:NaN', 1, 999999999, 0, 'Lunar Festival: Minions of Omen', 1, 0, 7, 10),
(91, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Tanaris', 1, 1, 9, 10),
(92, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Azshara', 1, 1, 9, 10),
(93, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Blasted Lands', 1, 1, 9, 10),
(94, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Eastern Plaguelands', 1, 1, 9, 10),
(95, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Attacking Burning Steppes', 1, 1, 9, 10),
(96, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - 50 Invasions Done', 1, 1, 9, 10),
(97, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - 100 Invasions Done', 1, 1, 9, 10),
(98, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - 150 Invasions Done', 1, 1, 9, 10),
(99, 'NaN-NaN-NaN NaN:NaN:NaN', '2037-01-01 00:59:59', 525600, 30240, 0, 'Scourge Invasion - Invasions Done', 1, 1, 9, 10),
(111, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Ironforge', 0, 1, 7, 10),
(112, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Stormwind', 0, 1, 7, 10),
(113, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Orgrimmar', 0, 1, 7, 10),
(114, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Thunder Bluff', 0, 1, 7, 10),
(115, '2021-02-16 22:00:00', '2037-02-11 23:00:00', 525600, 525600, 335, 'Love is in the Air - Popularity Contest Winner: Undercity', 0, 1, 7, 10);
/*!40000 ALTER TABLE `game_event` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;