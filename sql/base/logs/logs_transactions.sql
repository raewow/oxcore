-- MySQL dump
--
-- Table structure for table `logs_transactions`
--

DROP TABLE IF EXISTS `logs_transactions`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `logs_transactions` (
  `time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `type` enum('Bid','Buyout','PlaceAuction','Trade','Mail','MailCOD') DEFAULT NULL,
  `guid1` int unsigned NOT NULL DEFAULT '0',
  `money1` int unsigned NOT NULL DEFAULT '0',
  `spell1` int unsigned NOT NULL DEFAULT '0',
  `items1` varchar(255) NOT NULL DEFAULT '',
  `guid2` int unsigned NOT NULL DEFAULT '0',
  `money2` int unsigned NOT NULL DEFAULT '0',
  `spell2` int unsigned NOT NULL DEFAULT '0',
  `items2` varchar(255) NOT NULL DEFAULT '',
  KEY `guid2` (`guid2`),
  KEY `guid1` (`guid1`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;