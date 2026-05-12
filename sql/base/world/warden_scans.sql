-- MySQL dump
--
-- Table structure for table `warden_scans`
-- Table data for table `warden_scans`
--

DROP TABLE IF EXISTS `warden_scans`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `warden_scans` (
  `id` smallint unsigned NOT NULL AUTO_INCREMENT,
  `type` int DEFAULT '0',
  `str` text,
  `data` text,
  `address` int DEFAULT '0',
  `length` int DEFAULT '0',
  `result` tinytext NOT NULL,
  `flags` mediumint unsigned NOT NULL,
  `penalty` tinyint NOT NULL DEFAULT '-1' COMMENT 'Action to take if check fails',
  `build_min` smallint unsigned NOT NULL DEFAULT '5875',
  `build_max` smallint unsigned NOT NULL DEFAULT '5875',
  `comment` tinytext NOT NULL,
  UNIQUE KEY `id` (`id`)
) ENGINE=MyISAM AUTO_INCREMENT=98 DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `warden_scans`
--

LOCK TABLES `warden_scans` WRITE;
/*!40000 ALTER TABLE `warden_scans` DISABLE KEYS */;
INSERT INTO `warden_scans` (`id`, `type`, `str`, `data`, `address`, `length`, `result`, `flags`, `penalty`, `build_min`, `build_max`, `comment`) VALUES
(1, 0, NULL, NULL, 8679268, 6, '686561646572', 0, -1, 5875, 6005, 'Packet internal sign - "header"'),
(2, 0, NULL, NULL, 8696620, 6, '686561646572', 0, -1, 6141, 6141, 'Packet internal sign - "header"'),
(3, 0, NULL, NULL, 8530960, 6, '53595354454D', 0, -1, 5875, 6005, 'Packet internal sign - "SYSTEM"'),
(4, 0, NULL, NULL, 8547832, 6, '53595354454D', 0, -1, 6141, 6141, 'Packet internal sign - "SYSTEM"'),
(5, 2, NULL, '82D7E5CBC8D2F78A791E189BAB3FD5D4342BF7EB0CA3F129', 74044, NULL, '0', 0, -1, 4297, 6141, 'Cheat Engine dll'),
(6, 2, NULL, 'A444519CC419521B6D39990C1D95329C8D94B59226CBAA98', 16507, NULL, '0', 0, -1, 4297, 6141, 'WPE PRO dll'),
(7, 2, NULL, '3A0F8985E701343E439C74B675C72BBE2D8810A745569913', 372624, NULL, '0', 0, -1, 4297, 6141, 'rPE dll'),
(8, 0, NULL, NULL, 8151666, 4, 'D893FEC0', 0, -1, 5875, 5875, 'Jump gravity'),
(9, 0, NULL, NULL, 8151646, 2, '3075', 0, -1, 5875, 5875, 'Jump gravity water'),
(10, 0, NULL, NULL, 6382555, 2, '8A47', 0, -1, 5875, 5875, 'Anti root'),
(11, 0, NULL, NULL, 6380789, 1, 'F8', 0, -1, 5875, 5875, 'Anti move'),
(12, 0, NULL, NULL, 8151647, 1, '75', 0, -1, 5875, 5875, 'Anti jump'),
(13, 0, NULL, NULL, 8152026, 4, '8B4F7889', 0, -1, 5875, 5875, 'No fall damage'),
(14, 0, NULL, NULL, 6504892, 2, '7425', 0, -1, 5875, 5875, 'Super fly'),
(15, 0, NULL, NULL, 6383433, 2, '780F', 0, -1, 5875, 5875, 'Heartbeat interval speedhack'),
(16, 0, NULL, NULL, 6284623, 1, 'F4', 0, -1, 5875, 5875, 'Anti slow hack'),
(17, 0, NULL, NULL, 6504931, 2, '85D2', 0, -1, 5875, 5875, 'No fall damage'),
(18, 0, NULL, NULL, 8151565, 2, '2000', 0, -1, 5875, 5875, 'Fly hack'),
(19, 0, NULL, NULL, 7153475, 6, '890D509CCE00', 0, -1, 5875, 5875, 'No fog hack'),
(20, 0, NULL, NULL, 7138894, 6, 'A3D89BCE00EB', 0, -1, 5875, 5875, 'No fog hack'),
(21, 0, NULL, NULL, 7138907, 6, '890DD89BCE00', 0, -1, 5875, 5875, 'No fog hack'),
(22, 0, NULL, NULL, 6993044, 1, '74', 0, -1, 5875, 5875, 'WMO collision'),
(23, 0, NULL, NULL, 6502300, 1, 'FC', 0, -1, 5875, 5875, 'Wall climb'),
(24, 0, NULL, NULL, 6340512, 2, '7F7D', 0, -1, 5875, 5875, 'Looting hack'),
(25, 0, NULL, NULL, 6380455, 4, 'F4010000', 0, -1, 5875, 5875, 'Generic movement hack'),
(26, 0, NULL, NULL, 8151657, 4, '488C11C1', 0, -1, 5875, 5875, 'Water jump height hack'),
(27, 0, NULL, NULL, 6992319, 3, '894704', 0, -1, 5875, 5875, 'M2 collision'),
(28, 0, NULL, NULL, 6340529, 2, '746C', 0, -1, 5875, 5875, 'Looting hack'),
(29, 0, NULL, NULL, 6356016, 10, 'C70588D8C4000C000000', 0, -1, 5875, 5875, 'No water hack'),
(30, 0, NULL, NULL, 4730584, 6, '0F8CE1000000', 0, -1, 5875, 5875, 'Anti-Afk'),
(31, 0, NULL, NULL, 4803152, 7, 'A1C0EACE0085C0', 0, -1, 5875, 5875, 'noclip hack'),
(32, 0, NULL, NULL, 5946704, 6, '8BD18B0D80E0', 0, -1, 5875, 5875, 'M2 collision'),
(33, 0, NULL, NULL, 6340543, 2, '7546', 0, -1, 5875, 5875, 'M2 collision'),
(34, 0, NULL, NULL, 5341282, 1, '7F', 0, -1, 5875, 5875, 'Warden disable'),
(35, 0, NULL, NULL, 4989376, 1, '72', 0, -1, 5875, 5875, 'No fog hack'),
(36, 0, NULL, NULL, 8145237, 1, '8B', 0, -1, 5875, 5875, 'No fog hack'),
(37, 0, NULL, NULL, 6392083, 8, '8B450850E824DA1A', 0, -1, 5875, 5875, 'No fog hack'),
(38, 0, NULL, NULL, 8146241, 10, 'D9818C0000008BE55DC2', 0, -1, 5875, 5875, 'tp2plane hack'),
(39, 0, NULL, NULL, 6995731, 1, '74', 0, -1, 5875, 5875, 'Air swim hack'),
(40, 0, NULL, NULL, 6964859, 1, '75', 0, -1, 5875, 5875, 'Infinite jump hack'),
(41, 0, NULL, NULL, 6382558, 10, '84C074178B86A4000000', 0, -1, 5875, 5875, 'Gravity water hack'),
(42, 0, NULL, NULL, 8151997, 3, '895108', 0, -1, 5875, 5875, 'Gravity hack'),
(43, 0, NULL, NULL, 8152025, 1, '34', 0, -1, 5875, 5875, 'Plane teleport'),
(44, 0, NULL, NULL, 6516436, 1, 'FC', 0, -1, 5875, 5875, 'Zero fall time'),
(45, 0, NULL, NULL, 6501616, 1, 'FC', 0, -1, 5875, 5875, 'No fall damage'),
(46, 0, NULL, NULL, 6511674, 1, 'FC', 0, -1, 5875, 5875, 'Fall time hack'),
(47, 0, NULL, NULL, 6513048, 1, 'FC', 0, -1, 5875, 5875, 'Death bug hack'),
(48, 0, NULL, NULL, 6514072, 1, 'FC', 0, -1, 5875, 5875, 'Anti slow hack'),
(49, 0, NULL, NULL, 8152029, 3, '894E38', 0, -1, 5875, 5875, 'Anti slow hack'),
(50, 0, NULL, NULL, 4847346, 3, '8B45D4', 0, -1, 5875, 5875, 'Max camera distance hack'),
(51, 0, NULL, NULL, 4847069, 1, '74', 0, -1, 5875, 5875, 'Wall climb'),
(52, 0, NULL, NULL, 8155231, 3, '000000', 0, -1, 5875, 5875, 'Signature check'),
(53, 0, NULL, NULL, 6356849, 1, '74', 0, -1, 5875, 5875, 'Signature check'),
(54, 0, NULL, NULL, 6354889, 6, '0F8A71FFFFFF', 0, -1, 5875, 5875, 'Signature check'),
(55, 0, NULL, NULL, 4657642, 1, '74', 0, -1, 5875, 5875, 'Max interact distance hack'),
(56, 0, NULL, NULL, 6211360, 8, '558BEC83EC0C8B45', 0, -1, 5875, 5875, 'Hover speed hack'),
(57, 0, NULL, NULL, 8153504, 3, '558BEC', 0, -1, 5875, 5875, 'Flight speed hack'),
(58, 0, NULL, NULL, 6214285, 6, '8B82500E0000', 0, -1, 5875, 5875, 'Track all units hack'),
(59, 0, NULL, NULL, 8151558, 11, '25FFFFDFFB0D0020000089', 0, -1, 5875, 5875, 'No fall damage'),
(60, 0, NULL, NULL, 8155228, 6, '89868C000000', 0, -1, 5875, 5875, 'Run speed hack'),
(61, 0, NULL, NULL, 6356837, 2, '7474', 0, -1, 5875, 5875, 'Follow anything hack'),
(62, 0, NULL, NULL, 6751806, 1, '74', 0, -1, 5875, 5875, 'No water hack'),
(63, 0, NULL, NULL, 4657632, 2, '740A', 0, -1, 5875, 5875, 'Any name hack'),
(64, 0, NULL, NULL, 8151976, 4, '84E5FFFF', 0, -1, 5875, 5875, 'Plane teleport'),
(65, 0, NULL, NULL, 6214371, 6, '8BB1540E0000', 0, -1, 5875, 5875, 'Object tracking hack'),
(66, 0, NULL, NULL, 6818689, 5, 'A388F2C700', 0, -1, 5875, 5875, 'No water hack'),
(67, 0, NULL, NULL, 6186028, 5, 'C705ACD2C4', 0, -1, 5875, 5875, 'No fog hack'),
(68, 0, NULL, NULL, 5473808, 4, '30855300', 0, -1, 5875, 5875, 'Warden disable hack '),
(69, 0, NULL, NULL, 4208171, 3, '6B2C00', 0, -1, 5875, 5875, 'Warden disable hack'),
(70, 0, NULL, NULL, 7119285, 1, '74', 0, -1, 5875, 5875, 'Warden disable hack'),
(71, 0, NULL, NULL, 4729827, 1, '5E', 0, -1, 5875, 5875, 'Daylight hack'),
(72, 0, NULL, NULL, 6354512, 6, '0F84EA000000', 0, -1, 5875, 5875, 'Ranged attack stop hack'),
(73, 0, NULL, NULL, 5053463, 2, '7415', 0, -1, 5875, 5875, 'Officer note hack'),
(74, 4, 'World\\Lordaeron\\stratholme\\Activedoodads\\doors\\nox_door_plague.m2', NULL, 0, 0, 'B4452B6D95C98B186A70B008FA07BBAEF30DF7A2', 0, -1, 5464, 6141, 'Stratholme door'),
(75, 4, 'World\\Kalimdor\\onyxiaslair\\doors\\OnyxiasGate01.m2', NULL, 0, 0, '75195E4AEDA0BCAF048CA0E34D95A70D4F53C746', 0, -1, 4297, 6141, 'Onyxia gate'),
(76, 4, 'World\\Generic\\Human\\Activedoodads\\doors\\deadminedoor02.m2', NULL, 0, 0, '3DFF011B9AB134F37F885097E695351B91953564', 0, -1, 4297, 6141, 'Deadmines door'),
(77, 4, 'World\\Kalimdor\\silithus\\activedoodads\\ahnqirajdoor\\ahnqirajdoor02.m2', NULL, 0, 0, 'DBD4F407C468CC36134E621D160178FDA4D0D249', 0, -1, 5086, 6141, 'AQ door'),
(78, 4, 'World\\Kalimdor\\diremaul\\activedoodads\\doors\\diremaulsmallinstancedoor.m2', NULL, 0, 0, '0DC8DB46C85549C0FF1A600F6C236357C305781A', 0, -1, 4297, 6141, 'Dire Maul Gordok Inner Door'),
(79, 0, NULL, NULL, 8139737, 5, 'D84E14DEC1', 0, -1, 5875, 5875, 'UNKNOWN movement hack?'),
(80, 0, NULL, NULL, 8902804, 4, '8E977042', 0, -1, 5875, 5875, 'Wall climb hack'),
(81, 0, NULL, NULL, 8902808, 4, '0000E040', 0, -1, 5875, 5875, 'Run speed hack'),
(82, 0, NULL, NULL, 8154755, 7, '8166403FFFDFFF', 0, -1, 5875, 5875, 'Moveflag hack'),
(83, 0, NULL, NULL, 8445948, 4, 'BB8D243F', 0, -1, 5875, 5875, 'Wall climb hack'),
(84, 0, NULL, NULL, 6493717, 2, '741D', 0, -1, 5875, 5875, 'Speed hack'),
(85, 2, NULL, '33D233C9E887071B00E8', 13856, NULL, '1', 0, -1, 5875, 6005, 'Warden packet process code search sanity check'),
(86, 1, 'kernel32.dll', NULL, 0, 0, '1', 0, -1, 4297, 6141, 'Warden module search bypass sanity check'),
(87, 1, 'wpespy.dll', NULL, 0, 0, '0', 0, -1, 4297, 6141, 'WPE Pro'),
(88, 1, 'speedhack-i386.dll', NULL, 0, 0, '0', 0, -1, 4297, 6141, 'CheatEngine'),
(89, 1, 'tamia.dll', NULL, 0, 0, '0', 0, -1, 4297, 6141, 'Tamia hack'),
(90, 0, NULL, NULL, 12900744, 4, '0000C843', 0, -1, 5875, 5875, 'Nameplate extender'),
(91, 0, NULL, NULL, 8784512, 4, '00006144', 0, -1, 5875, 5875, 'Unlimited follow distance'),
(92, 0, NULL, NULL, 8423860, 4, 'DB0FC93F', 0, -1, 5875, 5875, 'FoV Hack'),
(93, 0, NULL, NULL, 5470880, 8, '558BEC8B156CD4C0', 0, -1, 5875, 6005, 'Packet Reading Hook'),
(94, 0, NULL, NULL, 5474448, 8, '558BEC8B15141BC1', 0, -1, 6141, 6141, 'Packet Reading Hook'),
(95, 0, NULL, NULL, 7119406, 8, 'A37889CE00C70574', 0, -1, 5875, 5875, 'ActivateNextModule change'),
(96, 0, NULL, NULL, 6392088, 8, '24DA1A005DC20800', 0, -1, 5875, 5875, 'InitMovementStatus change'),
(97, 4, 'World\\KhazModan\\Blackrock\\PassiveDoodads\\Doors\\BlackRockDoorSingle.m2', NULL, 0, 0, '2A43947CA91F92B6698A5286A7B883EFF967D6B4', 0, -1, 4297, 6141, 'UBRS door');
/*!40000 ALTER TABLE `warden_scans` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;