#!/usr/bin/env node
/**
 * Database introspection script
 * Compares current database structure against base tables and generates migrations
 *
 * Usage:
 *     node introspect_db.js [options]
 *
 * Options:
 *     --host HOST          MySQL host (default: 127.0.0.1)
 *     --port PORT          MySQL port (default: 3306)
 *     --user USER          MySQL user (default: root)
 *     --password PASSWORD  MySQL password (default: empty)
 *     --world-db DB        World database name (default: world)
 *     --logon-db DB        Auth/Logon database name (default: auth)
 *     --char-db DB         Characters database name (default: characters)
 *     --logs-db DB         Logs database name (default: logs)
 *     --base-dir DIR       Base SQL directory (default: _base)
 *     --migrations-dir DIR Migrations directory (default: migrations)
 *     --rebuild-base       Rebuild base table files from current database
 *     --exclude-data LIST  Comma-separated list of tables to exclude data from
 *                          (default: account,migrations)
 *                          Only structure will be exported for these tables
 */

import fs from "fs";
import path from "path";
import { createConnection as mysqlCreateConnection } from "mysql2/promise";

// Default connection settings
const DEFAULTS = {
  host: process.env.MYSQL_HOST || "127.0.0.1",
  port: parseInt(process.env.MYSQL_PORT || "3306", 10),
  user: process.env.MYSQL_USER || "root",
  password: process.env.MYSQL_PASSWORD || "",
  worldDb: process.env.WORLD_DB || "world",
  logonDb: process.env.LOGON_DB || "auth",
  charDb: process.env.CHAR_DB || "characters",
  logsDb: process.env.LOGS_DB || "logs",
  baseDir: "base",
  migrationsDir: "migrations",
  rebuildBase: false,
  excludeData: ["account", "realmcharacters", "migrations"], // Tables to exclude data from (structure only)
};

function parseArgs() {
  const args = process.argv.slice(2);
  const config = { ...DEFAULTS };
  let currentKey = null;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      const key = arg.substring(2).replace(/-/g, "");
      if (key === "rebuildbase") {
        config.rebuildBase = true;
        currentKey = null;
      } else {
        currentKey = key;
      }
    } else if (currentKey) {
      if (currentKey === "port") {
        config[currentKey] = parseInt(arg, 10);
      } else if (currentKey === "worlddb") {
        config.worldDb = arg;
      } else if (currentKey === "logondb") {
        config.logonDb = arg;
      } else if (currentKey === "chardb") {
        config.charDb = arg;
      } else if (currentKey === "logsdb") {
        config.logsDb = arg;
      } else if (currentKey === "basedir") {
        config.baseDir = arg;
      } else if (currentKey === "migrationsdir") {
        config.migrationsDir = arg;
      } else if (currentKey === "excludedata") {
        // Comma-separated list of table names
        config.excludeData = arg
          .split(",")
          .map((t) => t.trim())
          .filter((t) => t.length > 0);
      } else {
        config[currentKey] = arg;
      }
      currentKey = null;
    }
  }

  return config;
}

function createConnection(config, database) {
  const connectionConfig = {
    host: config.host,
    port: config.port,
    user: config.user,
    database: database,
    charset: "utf8mb4",
  };

  if (config.password && config.password.trim() !== "") {
    connectionConfig.password = config.password;
  }

  return mysqlCreateConnection(connectionConfig);
}

async function getTables(connection) {
  const [tables] = await connection.query(`
        SELECT TABLE_NAME 
        FROM information_schema.TABLES 
        WHERE TABLE_SCHEMA = DATABASE()
        AND TABLE_TYPE = 'BASE TABLE'
        ORDER BY TABLE_NAME
    `);
  return tables.map((row) => row.TABLE_NAME);
}

async function getCreateTable(connection, tableName) {
  const [rows] = await connection.query(`SHOW CREATE TABLE \`${tableName}\``);
  return rows[0]["Create Table"];
}

/**
 * Extract CREATE TABLE statement from base SQL file
 */
function extractCreateTableFromBaseFile(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  const content = fs.readFileSync(filePath, "utf8");

  // Find CREATE TABLE statement
  // It's usually after "DROP TABLE IF EXISTS" and before the semicolon
  const createTableMatch = content.match(/CREATE\s+TABLE\s+[^;]+;/is);
  if (!createTableMatch) {
    return null;
  }

  return createTableMatch[0].trim();
}

/**
 * Normalize CREATE TABLE statement for comparison
 * Removes comments, normalizes whitespace, and standardizes formatting
 */
function normalizeCreateTable(createTable) {
  if (!createTable) return "";

  // Remove MySQL conditional comments
  let normalized = createTable.replace(/\/\*![^*]+\*\/;?/g, "");

  // Remove single-line comments
  normalized = normalized.replace(/--[^\n]*/g, "");

  // Normalize whitespace
  normalized = normalized.replace(/\s+/g, " ");

  // Remove trailing semicolon
  normalized = normalized.replace(/;+\s*$/, "");

  // Normalize backticks and quotes
  normalized = normalized.replace(/`/g, "`");

  // Trim
  normalized = normalized.trim();

  return normalized;
}

/**
 * Compare two CREATE TABLE statements
 * Returns true if they are equivalent (ignoring formatting differences)
 */
function areTablesEqual(baseTable, currentTable) {
  const normalizedBase = normalizeCreateTable(baseTable);
  const normalizedCurrent = normalizeCreateTable(currentTable);

  // Simple string comparison after normalization
  // For more sophisticated comparison, we could parse the SQL
  return normalizedBase === normalizedCurrent;
}

/**
 * Get table structure differences
 * Returns an object with differences found
 */
function getTableDifferences(baseTable, currentTable) {
  const differences = {
    structureChanged: false,
    isNew: false,
    isRemoved: false,
    changes: [],
  };

  if (!baseTable && currentTable) {
    differences.isNew = true;
    differences.structureChanged = true;
    differences.changes.push("Table is new (not in base)");
    return differences;
  }

  if (baseTable && !currentTable) {
    differences.isRemoved = true;
    differences.structureChanged = true;
    differences.changes.push("Table was removed from database");
    return differences;
  }

  if (!areTablesEqual(baseTable, currentTable)) {
    differences.structureChanged = true;
    differences.changes.push("Table structure differs from base");
  }

  return differences;
}

/**
 * Generate migration SQL for table differences
 */
function generateMigrationSQL(tableName, differences, baseTable, currentTable) {
  const statements = [];

  if (differences.isNew) {
    // Table is new - create it using the current table structure
    if (currentTable) {
      // Remove the table name from CREATE TABLE to make it reusable
      // Extract just the table definition part
      let createStmt = currentTable;
      // Replace CREATE TABLE `name` with CREATE TABLE IF NOT EXISTS `name`
      createStmt = createStmt.replace(
        /CREATE\s+TABLE\s+`[^`]+`/i,
        `CREATE TABLE IF NOT EXISTS \`${tableName}\``
      );
      statements.push(`-- New table: \`${tableName}\``);
      statements.push(createStmt);
    }
  } else if (differences.isRemoved) {
    // Table was removed - drop it
    statements.push(`-- Removed table: \`${tableName}\``);
    statements.push(`DROP TABLE IF EXISTS \`${tableName}\`;`);
  } else if (differences.structureChanged) {
    // Structure changed - we need to generate ALTER TABLE statements
    // For now, we'll add a comment and include both structures for manual review
    // A more sophisticated implementation would parse and compare columns/indexes
    statements.push(`-- Table \`${tableName}\` structure differs from base`);
    statements.push(`-- Base structure:`);
    statements.push(
      `-- ${normalizeCreateTable(baseTable)
        .replace(/\n/g, " ")
        .substring(0, 200)}...`
    );
    statements.push(`-- Current structure:`);
    statements.push(
      `-- ${normalizeCreateTable(currentTable)
        .replace(/\n/g, " ")
        .substring(0, 200)}...`
    );
    statements.push(
      `-- Review differences and apply appropriate ALTER TABLE statements manually`
    );
  }

  return statements.filter((s) => s.trim().length > 0).join("\n");
}

/**
 * Generate migration file content
 */
function generateMigrationFile(migrationId, dbType, sqlStatements) {
  if (!sqlStatements || sqlStatements.trim().length === 0) {
    return null;
  }

  const lines = [];
  lines.push(`DROP PROCEDURE IF EXISTS add_migration;`);
  lines.push(`DELIMITER ??`);
  lines.push(`CREATE PROCEDURE \`add_migration\()`);
  lines.push(`BEGIN`);
  lines.push(`DECLARE v INT DEFAULT 1;`);
  lines.push(
    `SET v = (SELECT COUNT(*) FROM \`migrations\` WHERE \`id\`='${migrationId}');`
  );
  lines.push(`IF v = 0 THEN`);
  lines.push(`INSERT INTO \`migrations\` VALUES ('${migrationId}');`);
  lines.push(``);
  lines.push(sqlStatements);
  lines.push(``);
  lines.push(`-- End of migration.`);
  lines.push(`END IF;`);
  lines.push(`END??`);
  lines.push(`DELIMITER ;`);
  lines.push(`CALL add_migration();`);
  lines.push(`DROP PROCEDURE IF EXISTS add_migration;`);

  return lines.join("\n");
}

/**
 * Get migration ID (YYYYMMDDHHmmss format in UTC)
 */
function getMigrationId() {
  const now = new Date();
  const year = now.getUTCFullYear();
  const month = String(now.getUTCMonth() + 1).padStart(2, "0");
  const day = String(now.getUTCDate()).padStart(2, "0");
  const hour = String(now.getUTCHours()).padStart(2, "0");
  const minute = String(now.getUTCMinutes()).padStart(2, "0");
  const second = String(now.getUTCSeconds()).padStart(2, "0");

  return `${year}${month}${day}${hour}${minute}${second}`;
}

/**
 * Map database type to directory name
 */
function getDbTypeDir(dbType) {
  const mapping = {
    world: "world",
    auth: "auth",
    logon: "auth",
    characters: "characters",
    logs: "logs",
  };
  return mapping[dbType] || dbType;
}

/**
 * Get table data and generate INSERT statements
 */
async function getTableData(connection, tableName) {
  try {
    const [rows] = await connection.query(`SELECT * FROM \`${tableName}\``);
    if (rows.length === 0) {
      return null;
    }

    // Get column names
    const columns = Object.keys(rows[0]);

    // Generate INSERT statements (batch multiple rows per statement for efficiency)
    const insertStatements = [];
    const batchSize = 100; // Insert up to 100 rows per statement

    for (let i = 0; i < rows.length; i += batchSize) {
      const batch = rows.slice(i, i + batchSize);
      const values = batch.map((row) => {
        const rowValues = columns.map((col) => {
          const value = row[col];
          if (value === null || value === undefined) {
            return "NULL";
          } else if (typeof value === "string") {
            // Escape single quotes and backslashes
            const escaped = value.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
            return `'${escaped}'`;
          } else if (value instanceof Date) {
            // Format date as MySQL datetime
            const year = value.getFullYear();
            const month = String(value.getMonth() + 1).padStart(2, "0");
            const day = String(value.getDate()).padStart(2, "0");
            const hours = String(value.getHours()).padStart(2, "0");
            const minutes = String(value.getMinutes()).padStart(2, "0");
            const seconds = String(value.getSeconds()).padStart(2, "0");
            return `'${year}-${month}-${day} ${hours}:${minutes}:${seconds}'`;
          } else if (Buffer.isBuffer(value)) {
            // Handle binary data (BLOB)
            return `0x${value.toString("hex")}`;
          } else {
            return String(value);
          }
        });
        return `(${rowValues.join(", ")})`;
      });

      const columnList = columns.map((col) => `\`${col}\``).join(", ");
      insertStatements.push(
        `INSERT INTO \`${tableName}\` (${columnList}) VALUES\n${values.join(
          ",\n"
        )};`
      );
    }

    return insertStatements.join("\n\n");
  } catch (error) {
    console.log(
      `    ⚠ Warning: Could not export data for ${tableName}: ${error.message}`
    );
    return null;
  }
}

/**
 * Generate base SQL file content from CREATE TABLE statement
 */
function generateBaseTableFile(tableName, createTable, tableData = null) {
  const lines = [];

  // MySQL dump header
  lines.push(`-- MySQL dump`);
  lines.push(`--`);
  lines.push(`-- Table structure for table \`${tableName}\``);
  if (tableData) {
    lines.push(`-- Table data for table \`${tableName}\``);
  }
  lines.push(`--`);
  lines.push(``);

  // DROP TABLE statement
  lines.push(`DROP TABLE IF EXISTS \`${tableName}\`;`);

  // CREATE TABLE with MySQL conditional comments
  lines.push(`/*!40101 SET @saved_cs_client     = @@character_set_client */;`);
  lines.push(`/*!50503 SET character_set_client = utf8mb4 */;`);

  // Format CREATE TABLE statement - ensure it ends with semicolon
  let formattedCreate = createTable.trim();
  if (!formattedCreate.endsWith(";")) {
    formattedCreate += ";";
  }
  lines.push(formattedCreate);

  lines.push(`/*!40101 SET character_set_client = @saved_cs_client */;`);
  lines.push(``);

  // Add table data if provided
  if (tableData) {
    lines.push(`--`);
    lines.push(`-- Dumping data for table \`${tableName}\``);
    lines.push(`--`);
    lines.push(``);
    lines.push(`LOCK TABLES \`${tableName}\` WRITE;`);
    lines.push(`/*!40000 ALTER TABLE \`${tableName}\` DISABLE KEYS */;`);
    lines.push(tableData);
    lines.push(`/*!40000 ALTER TABLE \`${tableName}\` ENABLE KEYS */;`);
    lines.push(`UNLOCK TABLES;`);
    lines.push(``);
  }

  // Footer
  lines.push(`/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;`);
  lines.push(`/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;`);
  lines.push(`/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;`);
  lines.push(`/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;`);
  lines.push(
    `/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;`
  );
  lines.push(`/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;`);
  lines.push(`/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;`);

  return lines.join("\n");
}

/**
 * Rebuild base table files from current database
 */
async function rebuildBaseTables(config, dbType, dbName) {
  console.log(`\n${"=".repeat(60)}`);
  console.log(
    `Rebuilding base tables for ${dbType.toUpperCase()} database: ${dbName}`
  );
  console.log("=".repeat(60));

  const connection = await createConnection(config, dbName);

  try {
    const currentTables = await getTables(connection);
    console.log(`Found ${currentTables.length} tables in database`);

    // Get base directory
    const baseDir = path.join(config.baseDir, getDbTypeDir(dbType));
    console.log(`Base directory: ${baseDir}`);

    // Ensure base directory exists
    if (!fs.existsSync(baseDir)) {
      fs.mkdirSync(baseDir, { recursive: true });
      console.log(`Created base directory: ${baseDir}`);
    }

    let exportedCount = 0;
    let skippedCount = 0;
    let dataExportedCount = 0;
    let dataSkippedCount = 0;

    // For characters and logs databases, exclude ALL tables
    // For other databases, use the exclude list
    const excludeAllTables = dbType === "characters" || dbType === "logs";

    // Normalize exclude list to lowercase for case-insensitive comparison
    const excludeDataLower = excludeAllTables
      ? [] // Empty list - we'll exclude all tables anyway
      : (
          config.excludeData || ["account", "realmcharacters", "migrations"]
        ).map((t) => t.toLowerCase());

    // Export each table
    for (const tableName of currentTables) {
      // Skip migrations table - it's managed separately
      if (tableName === "migrations") {
        console.log(`  ⊘ ${tableName}: skipped (migrations table)`);
        skippedCount++;
        continue;
      }

      try {
        const createTable = await getCreateTable(connection, tableName);

        // Check if we should exclude data for this table
        // Always exclude data for characters and logs databases
        // Otherwise, check the exclude list
        const excludeData =
          excludeAllTables ||
          excludeDataLower.includes(tableName.toLowerCase());
        let tableData = null;

        if (!excludeData) {
          console.log(`    Fetching data for ${tableName}...`);
          tableData = await getTableData(connection, tableName);
          if (tableData) {
            dataExportedCount++;
            console.log(`    ✓ Data exported for ${tableName}`);
          } else {
            dataSkippedCount++;
            console.log(`    ⊘ No data to export for ${tableName}`);
          }
        } else {
          dataSkippedCount++;
          if (excludeAllTables) {
            console.log(
              `    ⊘ Data excluded for ${tableName} (all tables excluded for ${dbType} database)`
            );
          } else {
            console.log(
              `    ⊘ Data excluded for ${tableName} (in exclude list)`
            );
          }
        }

        const filePath = path.join(baseDir, `${tableName}.sql`);
        const fileContent = generateBaseTableFile(
          tableName,
          createTable,
          tableData
        );

        fs.writeFileSync(filePath, fileContent, "utf8");
        console.log(`  ✓ ${tableName}: exported to ${filePath}`);
        exportedCount++;
      } catch (error) {
        console.log(`  ✗ Error exporting ${tableName}: ${error.message}`);
      }
    }

    console.log(
      `\n✓ Exported ${exportedCount} table(s), skipped ${skippedCount} table(s)`
    );
    console.log(
      `  Data exported: ${dataExportedCount} table(s), excluded/skipped: ${dataSkippedCount} table(s)`
    );
    return {
      exported: exportedCount,
      skipped: skippedCount,
      dataExported: dataExportedCount,
      dataSkipped: dataSkippedCount,
    };
  } catch (error) {
    console.log(
      `ERROR: Failed to rebuild base tables for ${dbName}: ${error.message}`
    );
    throw error;
  } finally {
    await connection.end();
  }
}

/**
 * Process database and compare against base tables
 */
async function processDatabase(config, dbType, dbName) {
  console.log(`\n${"=".repeat(60)}`);
  console.log(`Processing ${dbType.toUpperCase()} database: ${dbName}`);
  console.log("=".repeat(60));

  const connection = await createConnection(config, dbName);

  try {
    const currentTables = await getTables(connection);
    console.log(`Found ${currentTables.length} tables in database`);

    // Get base directory
    const baseDir = path.join(config.baseDir, getDbTypeDir(dbType));
    console.log(`Base directory: ${baseDir}`);

    // Collect all differences
    const allDifferences = [];
    const baseTableMap = new Map();

    // Load base tables
    if (fs.existsSync(baseDir)) {
      const baseFiles = fs
        .readdirSync(baseDir)
        .filter((f) => f.endsWith(".sql"));
      console.log(`Found ${baseFiles.length} base SQL files`);

      for (const file of baseFiles) {
        const tableName = file.replace(/\.sql$/, "");
        const filePath = path.join(baseDir, file);
        const createTable = extractCreateTableFromBaseFile(filePath);
        if (createTable) {
          baseTableMap.set(tableName, createTable);
        }
      }
    } else {
      console.log(`Base directory does not exist: ${baseDir}`);
    }

    // Compare current tables with base
    for (const tableName of currentTables) {
      // Skip migrations table - it's managed separately
      if (tableName === "migrations") {
        console.log(`  ⊘ ${tableName}: skipped (migrations table)`);
        continue;
      }

      try {
        const currentCreateTable = await getCreateTable(connection, tableName);
        const baseCreateTable = baseTableMap.get(tableName) || null;

        const differences = getTableDifferences(
          baseCreateTable,
          currentCreateTable
        );

        if (differences.structureChanged || differences.isNew) {
          allDifferences.push({
            tableName,
            differences,
            baseTable: baseCreateTable,
            currentTable: currentCreateTable,
          });
          console.log(`  ⚠ ${tableName}: ${differences.changes.join(", ")}`);
        } else {
          console.log(`  ✓ ${tableName}: matches base`);
        }
      } catch (error) {
        console.log(`  ✗ Error processing ${tableName}: ${error.message}`);
      }
    }

    // Check for tables in base that don't exist in current database
    for (const [tableName, baseCreateTable] of baseTableMap.entries()) {
      // Skip migrations table
      if (tableName === "migrations") {
        continue;
      }

      if (!currentTables.includes(tableName)) {
        const differences = getTableDifferences(baseCreateTable, null);
        allDifferences.push({
          tableName,
          differences,
          baseTable: baseCreateTable,
          currentTable: null,
        });
        console.log(`  ⚠ ${tableName}: removed from database`);
      }
    }

    // Generate migration if there are differences
    if (allDifferences.length > 0) {
      console.log(`\nFound ${allDifferences.length} table(s) with differences`);

      // Generate migration SQL
      const migrationStatements = [];
      for (const diff of allDifferences) {
        const sql = generateMigrationSQL(
          diff.tableName,
          diff.differences,
          diff.baseTable,
          diff.currentTable
        );
        if (sql) {
          migrationStatements.push(
            `-- Changes for table \`${diff.tableName}\``
          );
          migrationStatements.push(sql);
          migrationStatements.push("");
        }
      }

      if (migrationStatements.length > 0) {
        const migrationId = getMigrationId();
        const migrationContent = generateMigrationFile(
          migrationId,
          dbType,
          migrationStatements.join("\n")
        );

        if (migrationContent) {
          // Ensure migrations directory exists
          if (!fs.existsSync(config.migrationsDir)) {
            fs.mkdirSync(config.migrationsDir, { recursive: true });
          }

          // Map dbType to migration file name format
          const migrationDbName = dbType === "logon" ? "logon" : dbType;
          const fileName = `${migrationId}_${migrationDbName}.sql`;
          const filePath = path.join(config.migrationsDir, fileName);

          fs.writeFileSync(filePath, migrationContent, "utf8");
          console.log(`\n✓ Migration file created: ${filePath}`);
          return {
            differences: allDifferences.length,
            migrationFile: filePath,
          };
        }
      }
    } else {
      console.log(`\n✓ No differences found - database matches base tables`);
    }

    return { differences: allDifferences.length, migrationFile: null };
  } catch (error) {
    console.log(`ERROR: Failed to process ${dbName}: ${error.message}`);
    throw error;
  } finally {
    await connection.end();
  }
}

async function main() {
  const config = parseArgs();

  // Check if mysql2 package is available
  try {
    await import("mysql2/promise");
  } catch (error) {
    console.log("ERROR: mysql2 package is not installed!");
    console.log("Please run: npm install mysql2");
    process.exit(1);
  }

  // Process all databases
  const databases = [
    { type: "world", name: config.worldDb },
    { type: "auth", name: config.logonDb },
    { type: "characters", name: config.charDb },
    { type: "logs", name: config.logsDb },
  ];

  if (config.rebuildBase) {
    // Rebuild base tables mode
    console.log("Database Base Tables Rebuilder");
    console.log("==============================");
    console.log(`Base directory: ${config.baseDir}`);

    let totalExported = 0;
    let totalSkipped = 0;
    let totalDataExported = 0;
    let totalDataSkipped = 0;

    // Show exclude list if configured
    if (config.excludeData && config.excludeData.length > 0) {
      console.log(
        `Tables excluded from data export: ${config.excludeData.join(", ")}`
      );
    }

    for (const db of databases) {
      try {
        const result = await rebuildBaseTables(config, db.type, db.name);
        totalExported += result.exported;
        totalSkipped += result.skipped;
        totalDataExported += result.dataExported || 0;
        totalDataSkipped += result.dataSkipped || 0;
      } catch (error) {
        console.error(
          `Failed to rebuild base tables for ${db.type}:`,
          error.message
        );
      }
    }

    console.log(`\n${"=".repeat(60)}`);
    console.log("Summary:");
    console.log(`  Tables exported: ${totalExported}`);
    console.log(`  Tables skipped: ${totalSkipped}`);
    console.log(`  Tables with data exported: ${totalDataExported}`);
    console.log(`  Tables with data excluded/skipped: ${totalDataSkipped}`);
    console.log("=".repeat(60));
  } else {
    // Normal migration generation mode
    console.log("Database Migration Generator");
    console.log("============================");
    console.log(`Base directory: ${config.baseDir}`);
    console.log(`Migrations directory: ${config.migrationsDir}`);

    let totalDifferences = 0;
    const migrationFiles = [];

    for (const db of databases) {
      try {
        const result = await processDatabase(config, db.type, db.name);
        totalDifferences += result.differences;
        if (result.migrationFile) {
          migrationFiles.push(result.migrationFile);
        }
      } catch (error) {
        console.error(`Failed to process ${db.type}:`, error.message);
      }
    }

    console.log(`\n${"=".repeat(60)}`);
    console.log("Summary:");
    console.log(`  Tables with differences: ${totalDifferences}`);
    console.log(`  Migration files created: ${migrationFiles.length}`);
    if (migrationFiles.length > 0) {
      console.log(`  Files:`);
      migrationFiles.forEach((file) => console.log(`    - ${file}`));
    }
    console.log("=".repeat(60));

    if (totalDifferences > 0 && migrationFiles.length === 0) {
      console.log(
        "\n⚠ Warning: Differences found but no migration file was created."
      );
      console.log(
        "This may indicate that differences could not be automatically migrated."
      );
      process.exit(1);
    }
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
