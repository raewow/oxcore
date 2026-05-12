-- MySQL dump
--
-- Table structure for table `spell_scripts`
-- Table data for table `spell_scripts`
--

DROP TABLE IF EXISTS `spell_scripts`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `spell_scripts` (
  `id` int unsigned NOT NULL DEFAULT '0',
  `delay` int unsigned NOT NULL DEFAULT '0',
  `priority` tinyint unsigned NOT NULL DEFAULT '0',
  `command` tinyint unsigned NOT NULL DEFAULT '0',
  `datalong` int unsigned NOT NULL DEFAULT '0',
  `datalong2` int unsigned NOT NULL DEFAULT '0',
  `datalong3` int unsigned NOT NULL DEFAULT '0',
  `datalong4` int unsigned NOT NULL DEFAULT '0',
  `target_param1` int unsigned NOT NULL DEFAULT '0',
  `target_param2` int unsigned NOT NULL DEFAULT '0',
  `target_type` tinyint unsigned NOT NULL DEFAULT '0',
  `data_flags` tinyint unsigned NOT NULL DEFAULT '0',
  `dataint` int NOT NULL DEFAULT '0',
  `dataint2` int NOT NULL DEFAULT '0',
  `dataint3` int NOT NULL DEFAULT '0',
  `dataint4` int NOT NULL DEFAULT '0',
  `x` float NOT NULL DEFAULT '0',
  `y` float NOT NULL DEFAULT '0',
  `z` float NOT NULL DEFAULT '0',
  `o` float NOT NULL DEFAULT '0',
  `condition_id` mediumint unsigned NOT NULL DEFAULT '0',
  `comments` varchar(255) NOT NULL
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `spell_scripts`
--

LOCK TABLES `spell_scripts` WRITE;
/*!40000 ALTER TABLE `spell_scripts` DISABLE KEYS */;
INSERT INTO `spell_scripts` (`id`, `delay`, `priority`, `command`, `datalong`, `datalong2`, `datalong3`, `datalong4`, `target_param1`, `target_param2`, `target_type`, `data_flags`, `dataint`, `dataint2`, `dataint3`, `dataint4`, `x`, `y`, `z`, `o`, `condition_id`, `comments`) VALUES
(20479, 1, 0, 15, 7, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Cast Spell suicide delay 1 sec'),
(24236, 3, 0, 15, 15589, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Cast Whirlwind (Mandokir)'),
(24236, 0, 0, 15, 20548, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Cast Root self (dure 3 sec)'),
(9055, 0, 0, 9, 3996126, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Spawn Bolt Charged Bramble pour quête Mage 1948'),
(22563, 0, 0, 6, 30, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, -1338, -293, 91.1, 0.09, 0, 'Fix TP: ALTERAC HORDE'),
(22564, 0, 0, 6, 30, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 632, -47, 43, 3.66, 0, 'Fix TP: ALTERAC ALLY'),
(11513, 0, 0, 17, 9284, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Create object: Full Leaden Collection Phial'),
(15794, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 5612, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Blackhand Dreadweaver: Say Emote Text'),
(15792, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 5614, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Blackhand Veteran: Say Emote Text'),
(24935, 2, 0, 15, 24936, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Cast RC Tank Control'),
(24935, 0, 0, 15, 24934, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Cast Summon RC Tank'),
(26532, 0, 0, 15, 26533, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Green Helper'),
(26541, 0, 0, 15, 26536, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Red Helper'),
(26469, 0, 0, 15, 26045, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Snowman'),
(26528, 0, 0, 15, 26529, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Reindeer'),
(13819, 0, 0, 15, 31726, 6, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Warhorse - Cast Spell Summon Holy Mount Visual'),
(23214, 0, 0, 15, 31726, 6, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Charger - Cast Spell Summon Holy Mount Visual'),
(5784, 0, 0, 15, 31725, 6, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Felsteed - Cast Spell Summon Nightmare'),
(23161, 0, 0, 15, 31725, 6, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Summon Dreadsteed - Cast Spell Summon Nightmare'),
(29137, 0, 0, 15, 29101, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Cleansing Flames - Create Flame of Stormwind'),
(29135, 0, 0, 15, 29102, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Cleansing Flames - Create Flame of Ironforge'),
(29126, 0, 0, 15, 29099, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Cleansing Flames - Create Flame of Darnassus'),
(29139, 0, 0, 15, 29133, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Cleansing Flames - Create Flame of Undercity'),
(29136, 0, 0, 15, 29130, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Cleansing Flames - Create Flame of Orgrimmar'),
(29138, 0, 0, 15, 29132, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Cleansing Flames - Create Flame of Thunder Bluff'),
(19028, 0, 0, 15, 18814, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Soul Link - Cast Soul Link Aura'),
(15702, 0, 0, 18, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Empty Pure Sample Jar - Despawn Creature'),
(26393, 0, 0, 15, 26394, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Elune\'s Blessing: Target - Cast Elune\'s BLessing (quest credit)'),
(26406, 0, 0, 15, 26454, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Stormwind - Teleport Alliance Player'),
(26406, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Stormwind - Port Horde Player to Moonglade'),
(26408, 0, 0, 15, 26452, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Ironforge - Teleport Alliance Player'),
(26408, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Ironforge - Port Horde Player to Moonglade'),
(26409, 0, 0, 15, 26450, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Darnassus - Teleport Alliance Player'),
(26409, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Darnassus - Port Horde Player to Moonglade'),
(26414, 0, 0, 15, 26453, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Orgrimmar - Teleport Horde Player'),
(26414, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Orgrimmar - Port Alliance Player to Moonglade'),
(26410, 0, 0, 15, 26456, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Undercity - Teleport Horde Player'),
(26410, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Undercity - Port Alliance Player to Moonglade'),
(26412, 0, 0, 15, 26455, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 'Lunar Teleport Cap: Thunderbluff - Teleport Horde Player'),
(26412, 0, 0, 15, 26451, 2, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 3, 'Lunar Teleport Cap: Thunderbluff - Port Alliance Player to Moonglade');
/*!40000 ALTER TABLE `spell_scripts` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;