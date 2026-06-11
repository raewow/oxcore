-- Creates oxcore databases on first MySQL container startup.
-- Base tables are applied by: cargo run --bin db -- migrate

CREATE DATABASE IF NOT EXISTS `auth` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE IF NOT EXISTS `world` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE IF NOT EXISTS `characters` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE IF NOT EXISTS `logs` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
