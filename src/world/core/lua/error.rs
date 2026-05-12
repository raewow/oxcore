//! Error types for the Lua scripting system.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for Lua operations.
pub type LuaResult<T> = Result<T, LuaError>;

/// Errors that can occur in the Lua scripting system.
#[derive(Error, Debug)]
pub enum LuaError {
    /// Lua runtime error.
    #[error("Lua error: {0}")]
    Runtime(#[from] mlua::Error),

    /// Script file not found.
    #[error("Script not found: {0}")]
    ScriptNotFound(PathBuf),

    /// Invalid script metadata.
    #[error("Invalid script metadata in {path}: {message}")]
    InvalidMetadata { path: PathBuf, message: String },

    /// Script registration failed.
    #[error("Failed to register script '{name}': {message}")]
    RegistrationFailed { name: String, message: String },

    /// Script callback error.
    #[error("Error in script '{script}' callback '{callback}': {message}")]
    CallbackError {
        script: String,
        callback: String,
        message: String,
    },

    /// Invalid action returned by script.
    #[error("Invalid action from script '{script}': {message}")]
    InvalidAction { script: String, message: String },

    /// IO error while loading scripts.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Script directory not found.
    #[error("Scripts directory not found: {0}")]
    ScriptsDirectoryNotFound(PathBuf),

    /// Reload in progress.
    #[error("Script reload already in progress")]
    ReloadInProgress,

    /// Script not registered.
    #[error("No script registered for {script_type} with id {id}")]
    NotRegistered { script_type: String, id: u32 },

    /// Manager already initialized.
    #[error("Lua script manager already initialized")]
    AlreadyInitialized,
}
