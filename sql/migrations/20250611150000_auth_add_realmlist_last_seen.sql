-- Add last_seen column for world server heartbeat tracking
ALTER TABLE `realmlist`
  ADD COLUMN `last_seen` timestamp NULL DEFAULT NULL AFTER `realmbuilds`;
