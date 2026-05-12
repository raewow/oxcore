#!/usr/bin/env node
/**
 * Setup base tables script
 * Runs all SQL files from sql/base/ directories to set up a fresh database
 *
 * Usage:
 *     node setup_base_tables.js [options]
 *
 * Interactive Mode (default):
 *     When run without --database option, the script will prompt you to:
 *     - Select which databases to setup (multiselect)
 *     - Confirm whether to drop databases first (yes/no)
 *
 * Non-Interactive Mode:
 *     Use --database option to skip interactive prompts
 *
 * Options:
 *     --host HOST          MySQL host (default: 127.0.0.1)
 *     --port PORT          MySQL port (default: 3306)
 *     --user USER          MySQL user (default: root)
 *     --password PASSWORD  MySQL password (default: empty)
 *     --world-db DB        World database name (default: world)
 *     --logon-db DB        Logon database name (default: auth)
 *     --char-db DB         Characters database name (default: characters)
 *     --logs-db DB         Logs database name (default: logs)
 *     --base-dir DIR       Base SQL directory (default: base)
 *     --database DB        Which database to setup: world, auth, characters, logs, or all (default: all, triggers interactive mode)
 *     --create-db          Create databases if they don't exist (default: false)
 *
 * Note:
 *     For better interactive prompts, install inquirer:
 *     npm install inquirer
 *     The script will fall back to basic readline prompts if inquirer is not available.
 */

import fs from "fs";
import path from "path";
import { createConnection as mysqlCreateConnection } from "mysql2/promise";
import readline from "readline";

// Try to use inquirer for better interactive prompts, fallback to readline
let inquirer;
async function loadInquirer() {
  if (inquirer !== undefined) {
    return inquirer; // Already loaded or attempted
  }

  try {
    const inq = await import("inquirer");
    inquirer = inq.default;
    return inquirer;
  } catch (error) {
    inquirer = null;
    return null;
  }
}

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
  database: "all",
  createDb: false,
};

function parseArgs() {
  const args = process.argv.slice(2);
  const config = { ...DEFAULTS };
  let currentKey = null;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      currentKey = arg.substring(2).replace(/-/g, "");
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
      } else if (currentKey === "database") {
        config.database = arg;
      } else if (currentKey === "createdb") {
        config.createDb = arg.toLowerCase() === "true";
      } else {
        config[currentKey] = arg;
      }
      currentKey = null;
    }
  }

  return config;
}

function createConnection(config, database = null) {
  const connectionConfig = {
    host: config.host,
    port: config.port,
    user: config.user,
    charset: "utf8mb4",
    multipleStatements: true, // Allow multiple statements per query
  };

  if (config.password && config.password.trim() !== "") {
    connectionConfig.password = config.password;
  }

  if (database) {
    connectionConfig.database = database;
  }

  return mysqlCreateConnection(connectionConfig);
}

async function createDatabaseIfNotExists(connection, dbName) {
  try {
    await connection.query(
      `CREATE DATABASE IF NOT EXISTS \`${dbName}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`
    );
    console.log(`  ✓ Database ${dbName} exists or was created`);
    return true;
  } catch (error) {
    console.log(`  ✗ Failed to create database ${dbName}: ${error.message}`);
    return false;
  }
}

async function dropDatabaseIfExists(connection, dbName) {
  try {
    await connection.query(`DROP DATABASE IF EXISTS \`${dbName}\``);
    console.log(`  ✓ Database ${dbName} dropped`);
    return true;
  } catch (error) {
    console.log(`  ✗ Failed to drop database ${dbName}: ${error.message}`);
    return false;
  }
}

function getSQLFiles(baseDir, dbType) {
  const dbDir = path.join(baseDir, dbType);

  if (!fs.existsSync(dbDir)) {
    console.log(`  ⚠ Directory not found: ${dbDir}`);
    return [];
  }

  const files = fs
    .readdirSync(dbDir)
    .filter((file) => file.endsWith(".sql"))
    .map((file) => ({
      name: file,
      path: path.join(dbDir, file),
      tableName: file.replace(".sql", ""),
    }))
    .sort((a, b) => a.name.localeCompare(b.name));

  return files;
}

async function executeSQLFile(connection, filePath, fileName) {
  try {
    // Ensure SQL_MODE allows zero dates before executing each file
    // This is needed because some SQL files may restore the old mode
    const [modeRows] = await connection.query(`SELECT @@SESSION.SQL_MODE as current_mode`);
    const currentMode = modeRows[0]?.current_mode || '';
    if (currentMode && typeof currentMode === 'string' && currentMode.includes('NO_ZERO_DATE')) {
      // Remove NO_ZERO_DATE if it's present
      const modes = currentMode.split(',').map(m => m.trim()).filter(m => 
        m !== 'NO_ZERO_DATE' && m !== 'NO_ZERO_IN_DATE' && m.length > 0
      );
      const newMode = modes.length > 0 ? modes.join(',') : '';
      await connection.query(`SET SESSION SQL_MODE = ?`, [newMode]);
    }
    
    const sql = fs.readFileSync(filePath, "utf8");

    // Execute the SQL
    await connection.query(sql);

    return { success: true, error: null };
  } catch (error) {
    // Some errors are acceptable (e.g., table already exists, duplicate key)
    const errorStr = error.message.toLowerCase();
    if (
      errorStr.includes("already exists") ||
      errorStr.includes("duplicate entry") ||
      errorStr.includes("duplicate key")
    ) {
      return { success: true, error: null, warning: error.message };
    }
    return { success: false, error: error.message };
  }
}

async function processDatabase(config, dbType, dbName, dropFirst = false) {
  console.log(`\n${"=".repeat(60)}`);
  console.log(`Setting up ${dbType.toUpperCase()} database: ${dbName}`);
  console.log("=".repeat(60));

  // Create connection without database first (to create/drop DB if needed)
  let connection = await createConnection(config);

  try {
    // Drop database if requested
    if (dropFirst) {
      console.log(`  Dropping database ${dbName}...`);
      await dropDatabaseIfExists(connection, dbName);
    }

    // Create database if needed
    if (config.createDb || dropFirst) {
      await createDatabaseIfNotExists(connection, dbName);
    }

    // Close and reconnect with database
    await connection.end();
    connection = await createConnection(config, dbName);

    // Initialize MySQL variables that may be referenced in SQL files
    // These are typically set at the start of MySQL dumps but individual files may reference them
    // Use COALESCE to ensure variables are never NULL (MySQL doesn't allow setting sql_mode to NULL)
    // Remove NO_ZERO_DATE to allow '0000-00-00 00:00:00' default values in old SQL files
    try {
      // First, save the old values
      await connection.query(`
        SET @OLD_SQL_MODE = COALESCE(@@SQL_MODE, '');
        SET @OLD_FOREIGN_KEY_CHECKS = COALESCE(@@FOREIGN_KEY_CHECKS, 0), FOREIGN_KEY_CHECKS = 0;
        SET @OLD_UNIQUE_CHECKS = COALESCE(@@UNIQUE_CHECKS, 0), UNIQUE_CHECKS = 0;
        SET @OLD_CHARACTER_SET_CLIENT = COALESCE(@@CHARACTER_SET_CLIENT, 'utf8mb4');
        SET @OLD_CHARACTER_SET_RESULTS = COALESCE(@@CHARACTER_SET_RESULTS, 'utf8mb4');
        SET @OLD_COLLATION_CONNECTION = COALESCE(@@COLLATION_CONNECTION, 'utf8mb4_unicode_ci');
        SET @OLD_SQL_NOTES = COALESCE(@@SQL_NOTES, 0), SQL_NOTES = 0;
      `);
      
      // Set SQL_MODE to allow zero dates (needed for old SQL files with '0000-00-00 00:00:00' defaults)
      // Get current sql_mode and remove NO_ZERO_DATE and NO_ZERO_IN_DATE
      const [rows] = await connection.query(`SELECT @@SESSION.SQL_MODE as current_mode`);
      let currentMode = rows[0]?.current_mode || '';
      
      if (currentMode && typeof currentMode === 'string') {
        // Remove NO_ZERO_DATE and NO_ZERO_IN_DATE from sql_mode
        const modes = currentMode.split(',').map(m => m.trim()).filter(m => 
          m !== 'NO_ZERO_DATE' && m !== 'NO_ZERO_IN_DATE' && m.length > 0
        );
        const newMode = modes.length > 0 ? modes.join(',') : '';
        if (newMode) {
          await connection.query(`SET SESSION SQL_MODE = ?`, [newMode]);
        } else {
          // If all modes were removed, set to empty string (allows zero dates)
          await connection.query(`SET SESSION SQL_MODE = ''`);
        }
      } else {
        // If sql_mode is empty or null, set it to empty string (allows zero dates)
        await connection.query(`SET SESSION SQL_MODE = ''`);
      }
    } catch (error) {
      // If initialization fails, log but continue (some SQL files may not need these)
      console.log(`  ⚠ Warning: Failed to initialize MySQL variables: ${error.message}`);
    }

    // Get SQL files
    const sqlFiles = getSQLFiles(config.baseDir, dbType);

    if (sqlFiles.length === 0) {
      console.log(`  ⚠ No SQL files found for ${dbType} database`);
      return { processed: 0, errors: 0, warnings: 0 };
    }

    console.log(`  Found ${sqlFiles.length} SQL file(s)`);

    let processed = 0;
    let errors = 0;
    let warnings = 0;

    // Execute each SQL file
    for (const file of sqlFiles) {
      console.log(`  Processing: ${file.name}...`);

      const result = await executeSQLFile(connection, file.path, file.name);

      if (result.success) {
        if (result.warning) {
          console.log(`    ⚠ Warning: ${result.warning}`);
          warnings++;
        } else {
          console.log(`    ✓ Success`);
        }
        processed++;
      } else {
        console.log(`    ✗ Error: ${result.error}`);
        errors++;

        // Ask if we should continue
        if (errors === 1) {
          const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
          });

          const answer = await new Promise((resolve) => {
            rl.question("Continue with remaining files? (y/n): ", resolve);
          });
          rl.close();

          if (answer.toLowerCase() !== "y") {
            break;
          }
        }
      }
    }

    console.log(
      `\n  Completed: ${processed} files processed, ${errors} errors, ${warnings} warnings`
    );
    return { processed, errors, warnings };
  } catch (error) {
    console.log(`ERROR: Failed to process ${dbName}: ${error.message}`);
    return { processed: 0, errors: 1, warnings: 0 };
  } finally {
    await connection.end();
  }
}

/**
 * Prompt user for database selection using inquirer or readline
 */
async function promptDatabaseSelection(availableDatabases) {
  const inq = await loadInquirer();
  if (inq) {
    // Use inquirer for better UX
    const { selectedDatabases } = await inq.prompt([
      {
        type: "checkbox",
        name: "selectedDatabases",
        message: "Select which databases you want to setup:",
        choices: availableDatabases.map((db) => ({
          name: `${db.type} (${db.name})`,
          value: db,
          checked: true, // Default to all selected
        })),
        validate: (answer) => {
          if (answer.length === 0) {
            return "You must select at least one database.";
          }
          return true;
        },
      },
    ]);
    return selectedDatabases;
  } else {
    // Fallback to readline for basic selection
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    console.log("\nAvailable databases:");
    availableDatabases.forEach((db, index) => {
      console.log(`  ${index + 1}. ${db.type} (${db.name})`);
    });

    const answer = await new Promise((resolve) => {
      rl.question(
        "\nEnter database numbers (comma-separated, or 'all' for all): ",
        resolve
      );
    });
    rl.close();

    if (answer.toLowerCase().trim() === "all") {
      return availableDatabases;
    }

    const indices = answer
      .split(",")
      .map((s) => parseInt(s.trim()) - 1)
      .filter((i) => !isNaN(i) && i >= 0 && i < availableDatabases.length);

    if (indices.length === 0) {
      console.log("No valid selections, using all databases");
      return availableDatabases;
    }

    return indices.map((i) => availableDatabases[i]);
  }
}

/**
 * Prompt user for drop confirmation
 */
async function promptDropDatabases() {
  const inq = await loadInquirer();
  if (inq) {
    const { dropFirst } = await inq.prompt([
      {
        type: "confirm",
        name: "dropFirst",
        message: "Do you want to drop the databases first?",
        default: false,
      },
    ]);
    return dropFirst;
  } else {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    const answer = await new Promise((resolve) => {
      rl.question(
        "Do you want to drop the databases first? (yes/no): ",
        resolve
      );
    });
    rl.close();

    return (
      answer.toLowerCase().trim() === "yes" ||
      answer.toLowerCase().trim() === "y"
    );
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

  // Check if base directory exists
  if (!fs.existsSync(config.baseDir)) {
    console.log(`ERROR: Base directory not found: ${config.baseDir}`);
    console.log("Please run introspect_db.js first to generate base SQL files");
    process.exit(1);
  }

  console.log("Setup Base Tables Script");
  console.log("========================");
  console.log(`Base directory: ${config.baseDir}`);

  // Build list of available databases
  const availableDatabases = [
    { type: "world", name: config.worldDb },
    { type: "auth", name: config.logonDb },
    { type: "characters", name: config.charDb },
    { type: "logs", name: config.logsDb },
  ];

  // Determine which databases to process
  let databases = [];
  let dropFirst = false;

  // If database is specified via command line, use that (non-interactive mode)
  if (config.database !== "all") {
    const dbMap = {
      world: config.worldDb,
      logon: config.logonDb,
      auth: config.logonDb,
      characters: config.charDb,
      logs: config.logsDb,
    };
    const dbType = config.database === "logon" ? "auth" : config.database;
    if (dbMap[config.database]) {
      databases.push({ type: dbType, name: dbMap[config.database] });
      dropFirst = false; // Don't drop in non-interactive mode unless explicitly set
    } else {
      console.log(`ERROR: Unknown database type: ${config.database}`);
      process.exit(1);
    }
  } else {
    // Interactive mode - prompt user
    const inq = await loadInquirer();
    if (!inq) {
      console.log(
        "\n⚠ Note: Install 'inquirer' package for better interactive prompts:"
      );
      console.log("   npm install inquirer");
      console.log("   Falling back to basic readline prompts\n");
    }

    // Prompt for database selection
    databases = await promptDatabaseSelection(availableDatabases);

    if (databases.length === 0) {
      console.log("No databases selected. Exiting.");
      process.exit(0);
    }

    // Prompt for drop confirmation
    dropFirst = await promptDropDatabases();
  }

  console.log(
    `\nSelected databases: ${databases
      .map((db) => `${db.type} (${db.name})`)
      .join(", ")}`
  );
  console.log(`Drop databases first: ${dropFirst ? "Yes" : "No"}`);
  console.log(
    `Create databases if needed: ${config.createDb || dropFirst ? "Yes" : "No"}`
  );

  let totalProcessed = 0;
  let totalErrors = 0;
  let totalWarnings = 0;

  for (const db of databases) {
    const result = await processDatabase(config, db.type, db.name, dropFirst);
    totalProcessed += result.processed;
    totalErrors += result.errors;
    totalWarnings += result.warnings;
  }

  console.log(`\n${"=".repeat(60)}`);
  console.log("Summary:");
  console.log(`  Files processed: ${totalProcessed}`);
  console.log(`  Errors: ${totalErrors}`);
  console.log(`  Warnings: ${totalWarnings}`);
  console.log("=".repeat(60));

  if (totalErrors > 0) {
    process.exit(1);
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
