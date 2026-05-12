-- MySQL dump
--
-- Table structure for table `script_texts`
-- Table data for table `script_texts`
--

DROP TABLE IF EXISTS `script_texts`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `script_texts` (
  `entry` mediumint NOT NULL,
  `content_default` text NOT NULL,
  `content_loc1` text,
  `content_loc2` text,
  `content_loc3` text,
  `content_loc4` text,
  `content_loc5` text,
  `content_loc6` text,
  `content_loc7` text,
  `content_loc8` text,
  `sound` mediumint unsigned NOT NULL DEFAULT '0',
  `type` tinyint unsigned NOT NULL DEFAULT '0',
  `language` tinyint unsigned NOT NULL DEFAULT '0',
  `emote` smallint unsigned NOT NULL DEFAULT '0',
  `comment` text,
  PRIMARY KEY (`entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Script Texts';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `script_texts`
--

LOCK TABLES `script_texts` WRITE;
/*!40000 ALTER TABLE `script_texts` DISABLE KEYS */;
INSERT INTO `script_texts` (`entry`, `content_default`, `content_loc1`, `content_loc2`, `content_loc3`, `content_loc4`, `content_loc5`, `content_loc6`, `content_loc7`, `content_loc8`, `sound`, `type`, `language`, `emote`, `comment`) VALUES
(-1000000, '<ScriptDev2 Text Entry Missing!>', NULL, '<Texte ScriptDev2 introuvable !>', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, 0, 'DEFAULT_TEXT'),
(-1409018, 'MY PATIENCE IS DWINDLING! COME GNATS TO YOUR DEATH!', NULL, 'MA PATIENCE S\'ÉPUISE... VENEZ VOUS FAIRE TUER MOUCHERONS !', NULL, NULL, NULL, NULL, NULL, NULL, 8048, 1, 0, 0, 'ragnaros SAY_MAGMABURST'),
(-1509018, 'I am rejuvinated!', NULL, 'Je reprends des forces !', NULL, NULL, NULL, NULL, NULL, NULL, 8593, 1, 0, 0, 'ossirian SAY_SURPREME1'),
(-1509019, 'My powers are renewed!', NULL, 'Mes pouvoirs reviennent !', NULL, NULL, NULL, NULL, NULL, NULL, 8595, 1, 0, 0, 'ossirian SAY_SURPREME2'),
(-1509020, 'My powers return!', NULL, 'Mes pouvoirs sont de retour !', NULL, NULL, NULL, NULL, NULL, NULL, 8596, 1, 0, 0, 'ossirian SAY_SURPREME3'),
(-1531001, 'Cower mortals! The age of darkness is at hand.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8616, 1, 0, 0, 'skeram SAY_AGGRO2'),
(-1531002, 'Tremble! The end is upon you.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8621, 1, 0, 0, 'skeram SAY_AGGRO3'),
(-1531004, 'Spineless wretches! You will drown in rivers of blood!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8619, 1, 0, 0, 'skeram SAY_SLAY2'),
(-1531005, 'The screams of the dying will fill the air. A symphony of terror is about to begin!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8620, 1, 0, 0, 'skeram SAY_SLAY3'),
(-1531006, 'Prepare for the return of the ancient ones!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8618, 1, 0, 0, 'skeram SAY_SPLIT'),
(-1533107, 'Enough! I grow tired of these distractions! ', NULL, 'Assez ! Je me lasse de ses passe-temps !', NULL, NULL, NULL, NULL, NULL, NULL, 9090, 1, 0, 0, 'kelthuzad SAY_SPECIAL3_MANA_DET'),
(-1533108, 'Fools, you have spread your powers too thin. Be free, my minions!', NULL, 'Imbéciles, vous avez éparpillés vos forces. Libérez-vous, mes serviteurs !', NULL, NULL, NULL, NULL, NULL, NULL, 9089, 1, 0, 0, 'kelthuzad SAY_SPECIAL2_DISPELL'),
(-1533118, 'Noo... o...', NULL, 'Noo... on...', NULL, NULL, NULL, NULL, NULL, NULL, 8828, 1, 0, 0, 'heigan SAY_DEATH'),
(-1780139, 'FOR THE HORDE!', NULL, 'POUR LA HORDE !', NULL, NULL, NULL, NULL, NULL, NULL, 0, 1, 1, 22, 'Silithus Field Duty Event - 7 (Horde)'),
(-1780138, 'Strength and Honor!', NULL, 'Force et Honneur !', NULL, NULL, NULL, NULL, NULL, NULL, 0, 1, 1, 22, 'Silithus Field Duty Event - 9 (Horde)'),
(-1780137, 'Shai, Merok! To my side!', NULL, 'Shai, Merok ! À mes côtés !', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 22, 'Silithus Field Duty Event - 6 (Horde)'),
(-1780136, 'Look! Something\'s coming!', NULL, 'Attention ! Quelque chose approche !', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 5, 'Silithus Field Duty Event - 8 (Horde)'),
(-1780135, 'Krug suddenly halts, his eyes caught by a motion down in the Hive.', NULL, 'Krug s\'interrompt soudainement, le regard attiré par un mouvement en contrebas.', NULL, NULL, NULL, NULL, NULL, NULL, 0, 2, 0, 0, 'Silithus Field Duty Event - 5 (Horde)'),
(-1780134, 'Be ready!  If I am to sign those papers, you\'re gonna have to...', NULL, 'Soyez sur vos gardes ! Si vous voulez que je signe ces papiers, il va falloir...', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 0, 'Silithus Field Duty Event - 4 (Horde)'),
(-1780133, 'Our scouts have reported that a Silithid attempt to take out this outpost is likely to occur very soon.', NULL, 'Les derniers rapports des éclaireurs nous laissent penser qu\'un assaut des Silithides sur cet avant-poste est imminent.', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 0, 'Silithus Field Duty Event - 3 (Horde)'),
(-1780132, 'Get into the ranks ! Fighting alongside the Orgrimmar Legion is no easy task. The recruits these elves from the Cenarion Circle keep sending us never last long.', NULL, 'Rentrez dans les rangs ! Se battre aux côtés de la Légion d\'Orgrimmar n\'est pas chose aisée. Les recrues que nous envoient ces elfes du Cercle Cénarien font rarement long feu.', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 0, 'Silithus Field Duty Event - 2 (Horde)'),
(-1780131, 'Lok\'tar Ogar !', NULL, 'Lok\'tar Ogar !', NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 1, 22, 'Silithus Field Duty Event - 1 (Horde)'),
(-1531019, 'It\'s too late to turn away.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8623, 1, 0, 0, 'veklor SAY_AGGRO_1'),
(-1531020, 'Prepare to embrace oblivion!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8626, 1, 0, 0, 'veklor SAY_AGGRO_2'),
(-1531021, 'Like a fly to the web.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8624, 1, 0, 0, 'veklor SAY_AGGRO_3'),
(-1388101, '%s shares his powers with his brethren.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 2, 0, 0, 'aq40 EMOTE_SHARE'),
(-1531022, 'Your brash arrogance!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8628, 1, 0, 0, 'veklor SAY_AGGRO_4'),
(-1531025, 'To decorate our halls!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8627, 1, 0, 0, 'veklor SAY_SPECIAL'),
(-1531026, 'Ah, lambs to the slaughter!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8630, 1, 0, 0, 'veknilash SAY_AGGRO_1'),
(-1531027, 'Let none survive!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8632, 1, 0, 0, 'veknilash SAY_AGGRO_2'),
(-1531028, 'Join me brother, there is blood to be shed!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8631, 1, 0, 0, 'veknilash SAY_AGGRO_3'),
(-1531029, 'Look brother, fresh blood!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8633, 1, 0, 0, 'veknilash SAY_AGGRO_4'),
(-1531032, 'Shall be your undoing!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8634, 3, 0, 0, 'veknilash SAY_SPECIAL'),
(-1531033, 'Death is close...', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8580, 4, 0, 0, 'cthun SAY_WHISPER_1'),
(-1531034, 'You are already dead.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8581, 4, 0, 0, 'cthun SAY_WHISPER_2'),
(-1531035, 'Your courage will fail.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8582, 4, 0, 0, 'cthun SAY_WHISPER_3'),
(-1531036, 'Your friends will abandon you.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8583, 4, 0, 0, 'cthun SAY_WHISPER_4'),
(-1531037, 'You will betray your friends.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8584, 4, 0, 0, 'cthun SAY_WHISPER_5'),
(-1531038, 'You will die.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8585, 4, 0, 0, 'cthun SAY_WHISPER_6'),
(-1531039, 'You are weak.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8586, 4, 0, 0, 'cthun SAY_WHISPER_7'),
(-1531040, 'Your heart will explode.', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 8587, 4, 0, 0, 'cthun SAY_WHISPER_8'),
(-1533134, 'A Guardian of Icecrown enters the fight!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 3, 0, 0, 'kelthuzad EMOTE_GUARDIAN'),
(-1533136, '%s teleports and begins to channel a spell!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 3, 0, 0, 'heigan EMOTE_TELEPORT'),
(-1533137, '%s rushes to attack once more!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 3, 0, 0, 'heigan EMOTE_RETURN'),
(-1533138, '%s teleports into the fray!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 3, 0, 0, 'gothik EMOTE_TO_FRAY'),
(-1533139, 'The central gate opens!', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 3, 0, 0, 'gothik EMOTE_GATE');
/*!40000 ALTER TABLE `script_texts` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;