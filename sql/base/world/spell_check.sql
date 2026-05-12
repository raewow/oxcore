-- MySQL dump
--
-- Table structure for table `spell_check`
-- Table data for table `spell_check`
--

DROP TABLE IF EXISTS `spell_check`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `spell_check` (
  `spellid` smallint unsigned NOT NULL DEFAULT '0',
  `SpellFamilyName` smallint NOT NULL DEFAULT '-1',
  `SpellFamilyMask` bigint NOT NULL DEFAULT '-1',
  `SpellIcon` int NOT NULL DEFAULT '-1',
  `SpellVisual` int NOT NULL DEFAULT '-1',
  `SpellCategory` int NOT NULL DEFAULT '-1',
  `EffectType` int NOT NULL DEFAULT '-1',
  `EffectAura` int NOT NULL DEFAULT '-1',
  `EffectIdx` tinyint NOT NULL DEFAULT '-1',
  `Name` varchar(40) NOT NULL DEFAULT '',
  `Code` varchar(40) NOT NULL DEFAULT '',
  PRIMARY KEY (`spellid`,`SpellFamilyName`,`SpellFamilyMask`,`SpellIcon`,`SpellVisual`,`SpellCategory`,`Code`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `spell_check`
--

LOCK TABLES `spell_check` WRITE;
/*!40000 ALTER TABLE `spell_check` DISABLE KEYS */;
INSERT INTO `spell_check` (`spellid`, `SpellFamilyName`, `SpellFamilyMask`, `SpellIcon`, `SpellVisual`, `SpellCategory`, `EffectType`, `EffectAura`, `EffectIdx`, `Name`, `Code`) VALUES
(18788, -1, -1, -1, -1, -1, 1, -1, -1, 'Demonic Sacrifice', 'Spell::EffectInstaKill'),
(18789, -1, -1, -1, -1, -1, -1, -1, -1, '', 'Spell::EffectInstaKill'),
(18790, -1, -1, -1, -1, -1, -1, -1, -1, '', 'Spell::EffectInstaKill'),
(18791, -1, -1, -1, -1, -1, -1, -1, -1, '', 'Spell::EffectInstaKill'),
(18792, -1, -1, -1, -1, -1, -1, -1, -1, '', 'Spell::EffectInstaKill');
/*!40000 ALTER TABLE `spell_check` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;