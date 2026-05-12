-- MySQL dump
--
-- Table structure for table `player_factionchange_spells`
-- Table data for table `player_factionchange_spells`
--

DROP TABLE IF EXISTS `player_factionchange_spells`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `player_factionchange_spells` (
  `alliance_id` smallint unsigned NOT NULL,
  `horde_id` smallint unsigned NOT NULL,
  `comment` varchar(255) NOT NULL DEFAULT '',
  PRIMARY KEY (`alliance_id`,`horde_id`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `player_factionchange_spells`
--

LOCK TABLES `player_factionchange_spells` WRITE;
/*!40000 ALTER TABLE `player_factionchange_spells` DISABLE KEYS */;
INSERT INTO `player_factionchange_spells` (`alliance_id`, `horde_id`, `comment`) VALUES
(458, 6654, ''),
(472, 580, ''),
(3561, 3567, 'Teleportation : Hurlevent / Orgrimmar'),
(3562, 3563, 'Teleportation : Forgefer / Fossoyeuse'),
(3565, 3566, 'Téléportation : Darnasssus / Téléportation : Pitons-du-Tonnerre'),
(6648, 6653, ''),
(6777, 8395, ''),
(6898, 10796, ''),
(6899, 10799, ''),
(10059, 11417, 'Portail : Hurlevent / Orgrimmar'),
(10789, 17464, ''),
(10793, 17463, ''),
(10969, 18990, ''),
(11416, 11418, ''),
(11419, 11420, ''),
(15779, 18992, ''),
(16082, 16080, ''),
(16083, 16081, ''),
(17453, 18989, ''),
(17459, 18991, ''),
(17460, 17450, ''),
(17461, 16084, ''),
(22717, 22724, ''),
(22719, 22718, ''),
(22720, 22721, ''),
(22723, 22722, ''),
(23219, 23246, ''),
(23222, 23247, ''),
(23225, 23249, ''),
(23227, 23251, ''),
(23228, 23252, ''),
(23229, 23250, ''),
(23238, 23243, ''),
(23239, 23241, ''),
(23240, 23242, ''),
(23338, 17465, ''),
(23510, 23509, '');
/*!40000 ALTER TABLE `player_factionchange_spells` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;