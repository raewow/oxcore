-- MySQL dump
--
-- Table structure for table `pool_gameobject_template`
-- Table data for table `pool_gameobject_template`
--

DROP TABLE IF EXISTS `pool_gameobject_template`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `pool_gameobject_template` (
  `id` int unsigned NOT NULL DEFAULT '0',
  `pool_entry` smallint unsigned NOT NULL DEFAULT '0',
  `chance` float unsigned NOT NULL DEFAULT '0',
  `description` varchar(255) NOT NULL,
  `flags` int unsigned NOT NULL DEFAULT '0' COMMENT 'FLAG_SPAWN_ENABLE_IF_WORLD_POP_OVER_BLIZZLIKE = 1',
  `patch_min` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Minimum content patch to load this entry',
  `patch_max` tinyint unsigned NOT NULL DEFAULT '10' COMMENT 'Maximum content patch to load this entry',
  PRIMARY KEY (`id`),
  KEY `pool_idx` (`pool_entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `pool_gameobject_template`
--

LOCK TABLES `pool_gameobject_template` WRITE;
/*!40000 ALTER TABLE `pool_gameobject_template` DISABLE KEYS */;
INSERT INTO `pool_gameobject_template` (`id`, `pool_entry`, `chance`, `description`, `flags`, `patch_min`, `patch_max`) VALUES
(180753, 42905, 0, 'Azshara - Patch of Elemental Water', 0, 7, 10),
(175785, 22909, 0, 'LBRS - Inconspicuous Documents', 0, 0, 10),
(1727, 1339, 0, 'Hillsbrad Foothills - Dun Garok - Keg of Shindigger Stout (1727)', 0, 0, 10),
(2744, 1342, 0, 'Stranglethorn Vale - Giant Clam', 0, 0, 10),
(375, 1597, 0, 'Tirisfal Glades - Tirisfal Pumpkin', 0, 0, 10),
(152621, 21604, 0, 'Azshara - Azsharite Formation', 0, 0, 10),
(152620, 21603, 0, 'Azshara - Azsharite Formation', 0, 0, 10),
(175334, 139, 0, 'LBRS - Bijous Belongings', 0, 0, 10),
(161557, 140, 0, 'Elwynn Forest - Milly\'s Harvest', 0, 0, 10),
(175966, 141, 0, 'Stratholme - Enchanted Scarlet Thread', 0, 0, 10),
(154357, 142, 0, 'Redridge Mountains - Glinting Mud', 0, 0, 10),
(176249, 143, 0, 'Stratholme - Scourge Data', 0, 0, 10),
(2086, 144, 0, 'Stranglethorn Vale - Bloodsail Charts', 0, 0, 10),
(2087, 150, 0, 'Stranglethorn Vale - Bloodsail Orders', 0, 0, 10),
(179548, 1596, 0, 'Dire Maul - Dusty Tome', 0, 1, 10),
(176753, 1010, 0, 'Tirisfal Glades - Doom Weed', 0, 0, 10),
(175566, 1009, 0, 'Tirisfal Glades - Gloom Weed', 0, 0, 10),
(181287, 1595, 0, 'Naxxramas - Frozen Rune', 0, 9, 10),
(1673, 1613, 0, 'Teldrassil - Fel Cone', 0, 0, 10),
(4608, 1614, 0, 'Teldrassil - Timberling Sprout', 0, 0, 10),
(3685, 1616, 0, 'The Barrens - Silithid Mound', 0, 0, 10),
(1723, 1617, 0, 'Hillsbrad Foothills - Mudsnout Blossom', 0, 0, 10),
(175928, 1618, 0, 'Thousand Needles - Incendia Agarve', 0, 0, 10),
(2724, 1619, 0, 'Westfall - Sack of Oats', 0, 0, 10),
(13360, 1636, 0, 'Darkshore - Mathystra Relic (Entry: 13360)', 0, 0, 10),
(12654, 1636, 0, 'Darkshore - Mathystra Relic (Entry: 12654)', 0, 0, 10),
(13872, 1636, 0, 'Darkshore - Mathystra Relic (Entry: 13872)', 0, 0, 10),
(24798, 1620, 0, 'Swamp of Sorrows - Sundried Driftwood', 0, 0, 10),
(28604, 1621, 0, 'Swamp of Sorrows - Scattered Crate', 0, 0, 10),
(86492, 1622, 0, 'Darkshore - Crate of Elunite', 0, 0, 10),
(89634, 1623, 0, 'Wetlands - Iron Coral', 0, 0, 10),
(89635, 1624, 0, 'Thousand Needles - Sunscorched Shell', 0, 0, 10),
(141931, 1625, 0, 'Feralas - Hippogryph Egg', 0, 0, 10),
(148516, 1626, 0, 'Azshara - Tablet of Beth\'Amara', 0, 0, 10),
(148513, 1627, 0, 'Azshara - Tablet of Jinyael', 0, 0, 10),
(148514, 1628, 0, 'Azshara - Tablet of Markri', 0, 0, 10),
(148515, 1629, 0, 'Azshara - Tablet of Saelhai', 0, 0, 10),
(152094, 1630, 0, 'Teldrassil - Hyacinth Mushroom', 0, 0, 10),
(11713, 18858, 0, 'Darkshore - Death Cap', 0, 0, 10),
(271, 18857, 0, 'Loch Modan - Miners League Crates', 0, 0, 10),
(178195, 18856, 0, 'Ashenvale - Warsong Oils', 0, 0, 10),
(140971, 14, 0, 'Tanaris - Gahz\'ridian', 0, 0, 10),
(3236, 18855, 0, 'Durotar - Gnomish Toolbox', 0, 0, 10),
(180658, 3292, 0, 'Barrens - School of Deviate Fish', 0, 7, 10),
(176116, 3012, 0, 'Eastern Plaguelands - Pamela\'s Doll\'s Head', 0, 0, 10),
(176142, 3013, 0, 'Eastern Plaguelands - Pamela\'s Doll\'s Left Side', 0, 0, 10),
(176143, 3014, 0, 'Eastern Plaguelands - Pamela\'s Doll\'s Right Side', 0, 0, 10),
(2560, 1635, 0, 'Stranglethorn Vale - Half Buried Bottle', 0, 0, 10),
(142191, 41, 0, 'Hinterlands - Horde Supply Crate', 0, 0, 10),
(19016, 18859, 0, 'Ashenvale - Stardust Covered Bush', 0, 0, 10),
(276, 18860, 0, 'Dun Morogh - Shimmerweed Basket', 0, 0, 10),
(153239, 18861, 0, 'Hinterlands - Wildkin Feather', 0, 0, 10),
(177464, 18862, 0, 'Eastern Plaguelands - Large Termite Mound', 0, 0, 10),
(2912, 18863, 0, 'Mulgore - Ambercorn', 0, 0, 10),
(2068, 18864, 0, 'Hinterlands - Pupellyverbos Port', 0, 0, 10),
(177785, 18865, 0, 'Moonglade - Bauble Container', 0, 0, 10),
(152095, 18866, 0, 'Teldrassil - Moonpetal Lily', 0, 0, 10),
(178185, 18867, 0, 'Blackfathom Deeps - Sapphire of Aku\'Mai (178185)', 0, 0, 10),
(177726, 18868, 0, 'Western Plaguelands - Tool Bucket', 0, 0, 10),
(11714, 18869, 0, 'Darkshore - Scaber Stalk', 0, 0, 10),
(2712, 21551, 0, 'Arathi Highlands - Calcified Elven Gems', 0, 0, 10),
(2743, 63, 0, 'Badlands - Carved Stone Urn', 0, 0, 10),
(2910, 64, 0, 'Mulgore - Well Stone', 0, 0, 10),
(3240, 21552, 0, 'Durotar - Taillasher Eggs', 0, 0, 10),
(3290, 95, 0, 'Durotar - Stolen Supply Sacks', 0, 0, 10),
(22246, 96, 0, 'Desolace - Tear of Theradras', 0, 0, 10),
(153123, 97, 0, 'Azshara - Kim\'jael\'s Equipment', 0, 0, 10),
(157936, 98, 0, 'UnGoro Crater - UnGoro Dirt Pile', 0, 0, 10),
(161527, 99, 0, 'UnGoro Crater - Dinosaur Bone', 0, 0, 10),
(164662, 100, 0, 'Tirisfal Glades - Equipment Boxes', 0, 0, 10),
(161752, 101, 0, 'Barrens - Tool Bucket', 0, 0, 10),
(164958, 102, 0, 'UnGoro Crater - Bloodpetal Sprout', 0, 0, 10),
(175324, 103, 0, 'Winterspring - Frostmaul Shards', 0, 0, 10),
(175384, 145, 0, 'Thousand Needles - Highperch Wyvern Egg', 0, 0, 10),
(175708, 21553, 0, 'Barrens - Crossroads Supply Crates', 0, 0, 10),
(176630, 147, 0, 'Arathi Highlands - Keepsake of Remembrance', 0, 0, 10),
(176793, 148, 0, 'Elwynn Forest - Bundle of Wood', 0, 0, 10),
(177926, 149, 0, 'Stonetalon Mountains - Gaea Seed', 0, 0, 10),
(178144, 153, 0, 'Ashenvale - Troll Chest', 0, 0, 10),
(178184, 154, 0, 'Blackfathom Deeps - Sapphire of Aku\'Mais (178184)', 0, 0, 10),
(178186, 157, 0, 'Blackfathom Deeps - Sapphire of Aku\'Mais (178186)', 0, 0, 10),
(179922, 155, 0, 'Hinterlands - Vessel of Tainted Blood', 0, 3, 10),
(179908, 158, 0, 'Hinterlands - Slagtree\'s Lost Tools', 0, 3, 10),
(181053, 159, 0, 'Dustwallow Marsh - Basket of Bloodkelp', 0, 8, 10),
(181098, 160, 0, 'Burning Steppes - Volcanic Ash', 0, 8, 10),
(153556, 161, 0, 'Burning Steppes - Thaurissan Relic', 0, 0, 10),
(17282, 250, 0, 'Ashenvale - Plant Bundle', 0, 0, 10),
(142344, 251, 0, 'Artificial Extrapolator - Gnomeregan', 0, 0, 10),
(19903, 21601, 0, 'Indurium Mineral Vein - Uldaman', 0, 0, 10),
(180215, 21401, 0, 'Zul\'Gurub - Hakkari Thorium Vein', 0, 5, 10),
(181598, 76, 0, 'Silithus - Silithyst Geyser', 0, 10, 10),
(175565, 3585, 0, 'Thousand Needles - Alien Egg', 0, 0, 10),
(175802, 3587, 0, 'Western Plaguelands - Small Lockbox', 0, 0, 10),
(175407, 3588, 0, 'Winterspring - Moontouched Feather', 0, 0, 10),
(22550, 4902, 0, 'Swamp of Sorrows - Draenethyst Crystals', 0, 0, 10),
(2714, 4903, 0, 'Hillsbrad Foothills - Alterac Granite', 0, 1, 10),
(177750, 4904, 0, 'Darkshore - Lunar Fungal Bloom', 0, 1, 10),
(141853, 4905, 0, 'Hinterlands - Violet Tragan', 0, 1, 10),
(22245, 4906, 0, 'Desolace - Sack of Meat', 0, 1, 10),
(19015, 4907, 0, 'Ashenvale - Elunes Tear', 0, 1, 10);
/*!40000 ALTER TABLE `pool_gameobject_template` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;