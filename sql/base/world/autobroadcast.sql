-- MySQL dump
--
-- Table structure for table `autobroadcast`
--

DROP TABLE IF EXISTS `autobroadcast`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `autobroadcast` (
  `id` int unsigned NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `string_id` int DEFAULT NULL COMMENT 'String ID from mangos_string table',
  `schedule` varchar(255) NOT NULL COMMENT 'Cron schedule format: 5-field (minute hour day month weekday) or 6-field (second minute hour day month weekday). Examples: "* * * * *" (every minute), "0 */10 * * * *" (every 10 minutes)',
  `enabled` tinyint unsigned NOT NULL DEFAULT '1' COMMENT 'Enable/disable this broadcast (0=disabled, 1=enabled)',
  PRIMARY KEY (`id`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `autobroadcast`
--

LOCK TABLES `autobroadcast` WRITE;
/*!40000 ALTER TABLE `autobroadcast` DISABLE KEYS */;

-- Example: Broadcast every 1 minute
-- Note: You need to have a corresponding entry in mangos_string table
-- For example, if you want to use string_id 1000, first add it to mangos_string:
-- INSERT INTO `mangos_string` (`entry`, `content_default`) VALUES (1000, 'This is a test autobroadcast message!');
-- Then uncomment the line below:
-- INSERT INTO `autobroadcast` (`string_id`, `schedule`, `enabled`) VALUES (1000, '* * * * *', 1);  -- Every minute (5-field format, auto-converted to 6-field)

/*!40000 ALTER TABLE `autobroadcast` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;