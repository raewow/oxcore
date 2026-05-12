//! Script loader and discovery.
//!
//! This module handles scanning the scripts directory, loading Lua files,
//! and executing them in the sandbox environment.

use super::api::{parse_metadata, ScriptMetadata, ScriptRegistry};
use super::error::{LuaError, LuaResult};
use mlua::{Lua, Table};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Information about a loaded script file.
#[derive(Debug, Clone)]
pub struct LoadedScript {
    pub path: PathBuf,
    pub metadata: ScriptMetadata,
    pub content: String,
}

/// Result of loading scripts.
#[derive(Debug, Clone, Default)]
pub struct LoadResult {
    pub loaded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

impl LoadResult {
    pub fn success(&self) -> bool {
        self.failed == 0
    }
}

/// Discover all Lua script files in a directory.
pub fn discover_scripts(scripts_dir: &Path) -> LuaResult<Vec<PathBuf>> {
    if !scripts_dir.exists() {
        return Err(LuaError::ScriptsDirectoryNotFound(
            scripts_dir.to_path_buf(),
        ));
    }

    let mut scripts = Vec::new();
    discover_scripts_recursive(scripts_dir, &mut scripts)?;

    // Sort by path for deterministic load order
    scripts.sort();

    Ok(scripts)
}

fn discover_scripts_recursive(dir: &Path, scripts: &mut Vec<PathBuf>) -> LuaResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            discover_scripts_recursive(&path, scripts)?;
        } else if path.extension().map(|e| e == "lua").unwrap_or(false) {
            scripts.push(path);
        }
    }

    Ok(())
}

/// Load and parse a script file.
pub fn load_script_file(path: &Path) -> LuaResult<LoadedScript> {
    let content = fs::read_to_string(path)?;
    let metadata = parse_metadata(&content);

    Ok(LoadedScript {
        path: path.to_path_buf(),
        metadata,
        content,
    })
}

/// Load all scripts from a directory into the Lua VM.
pub fn load_all_scripts(
    lua: &Lua,
    sandbox: &Table,
    scripts_dir: &Path,
    registry: Arc<ScriptRegistry>,
) -> LuaResult<LoadResult> {
    let mut result = LoadResult::default();

    // Discover script files
    let script_paths = match discover_scripts(scripts_dir) {
        Ok(paths) => paths,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to discover scripts: {}", e));
            result.failed = 1;
            return Ok(result);
        }
    };

    tracing::debug!("Discovered {} Lua script files", script_paths.len());

    // Load and parse all scripts
    let mut scripts: Vec<LoadedScript> = Vec::new();
    for path in script_paths {
        match load_script_file(&path) {
            Ok(script) => scripts.push(script),
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to load {:?}: {}", path, e));
                result.failed += 1;
            }
        }
    }

    // Sort by priority (lower = loaded first)
    scripts.sort_by_key(|s| s.metadata.priority);

    // Look for _init.lua first
    let init_script = scripts.iter().find(|s| {
        s.path
            .file_name()
            .map(|n| n == "_init.lua")
            .unwrap_or(false)
    });

    // Execute _init.lua first if it exists
    if let Some(init) = init_script {
        if let Err(e) = execute_script(lua, sandbox, init) {
            result
                .errors
                .push(format!("Failed to execute _init.lua: {}", e));
            result.failed += 1;
        } else {
            result.loaded += 1;
            tracing::debug!("Loaded _init.lua");
        }
    }

    // Execute all other scripts
    for script in &scripts {
        // Skip _init.lua (already loaded)
        if script
            .path
            .file_name()
            .map(|n| n == "_init.lua")
            .unwrap_or(false)
        {
            continue;
        }

        match execute_script(lua, sandbox, script) {
            Ok(_) => {
                result.loaded += 1;
                tracing::debug!("Loaded script: {:?}", script.path);
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to execute {:?}: {}", script.path, e));
                result.failed += 1;
            }
        }
    }

    tracing::info!(
        "Loaded {} scripts ({} failed)",
        result.loaded,
        result.failed
    );

    Ok(result)
}

/// Execute a single script in the sandbox.
fn execute_script(lua: &Lua, sandbox: &Table, script: &LoadedScript) -> LuaResult<()> {
    // Compile the script
    let chunk = lua
        .load(&script.content)
        .set_name(script.path.to_string_lossy())
        .set_environment(sandbox.clone());

    // Execute it
    chunk.exec().map_err(|e| LuaError::Runtime(e))?;

    Ok(())
}

/// Get the default scripts directory path.
/// Scripts are stored at the project root /scripts/ folder, not in data_dir.
pub fn default_scripts_path(data_dir: &Path) -> PathBuf {
    // Canonicalize data_dir so relative paths like "data" resolve properly
    let data_dir = data_dir.canonicalize().unwrap_or_else(|_| data_dir.to_path_buf());

    // Go up from data_dir to find the project root, then use /scripts/
    // data_dir is typically "<project_root>/server/data", so we go up TWO levels
    if let Some(parent) = data_dir.parent() {
        // parent is "server/"
        if let Some(grandparent) = parent.parent() {
            // grandparent is project root
            let scripts = grandparent.join("scripts");
            if scripts.exists() {
                return scripts;
            }
        }
        // Try sibling of parent
        let scripts = parent.join("scripts");
        if scripts.exists() {
            return scripts;
        }
    }

    // Fallback: try current working directory
    let cwd_scripts = PathBuf::from("scripts");
    if cwd_scripts.exists() {
        return cwd_scripts;
    }

    // Final fallback (will produce a "not found" error in discover_scripts)
    PathBuf::from("scripts")
}
