//! MPQ Archive Handling
//!
//! This module provides an interface for reading MPQ (Mo'PaQ) archives.
//! MPQ is Blizzard's archive format used in World of Warcraft.

use anyhow::{Context, Result};
use wow_mpq::Archive;
use std::path::{Path, PathBuf};

/// MPQ Archive
pub struct MpqArchive {
    path: PathBuf,
    pub(crate) archive: Archive,
}

impl MpqArchive {
    /// Open an MPQ archive
    pub fn open(path: &Path) -> Result<Self> {
        // Validate file exists
        if !path.exists() {
            anyhow::bail!("MPQ file does not exist: {}", path.display());
        }

        // Open MPQ archive using wow-mpq
        let archive = Archive::open(path)
            .with_context(|| format!("Failed to open MPQ archive: {}", path.display()))?;

        Ok(Self {
            path: path.to_path_buf(),
            archive,
        })
    }

    /// List all files in the archive
    pub fn list_files(&mut self) -> Result<Vec<String>> {
        // wow-mpq's list() method returns entries
        let entries = self.archive.list()
            .with_context(|| format!("Failed to list files in archive: {}", self.path.display()))?;
        Ok(entries.iter().map(|e| e.name.clone()).collect())
    }

    /// Check if a file exists in the archive
    pub fn has_file(&mut self, file_path: &str) -> bool {
        // Try to list and check if file exists
        // Note: This is inefficient but wow-mpq doesn't have a direct contains() method
        self.archive.list()
            .map(|entries| entries.iter().any(|e| e.name == file_path))
            .unwrap_or(false)
    }

    /// Extract a file from the archive
    pub fn extract_file(&mut self, file_path: &str, output_path: &Path) -> Result<bool> {
        // Try to read the file from the archive
        match self.archive.read_file(file_path) {
            Ok(data) => {
                // Write the file to the output path
                std::fs::write(output_path, data)
                    .with_context(|| format!("Failed to write file: {}", output_path.display()))?;
                Ok(true)
            }
            Err(_) => {
                // File not in this archive
                Ok(false)
            }
        }
    }
}

/// MPQ File (for reading individual files)
pub struct MpqFile {
    data: Vec<u8>,
    position: usize,
}

impl MpqFile {
    /// Create a new MPQ file from data
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            position: 0,
        }
    }

    /// Read bytes from the file
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining = self.data.len() - self.position;
        let to_read = buf.len().min(remaining);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
            self.position += to_read;
        }

        to_read
    }

    /// Get the size of the file
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Check if at end of file
    pub fn is_eof(&self) -> bool {
        self.position >= self.data.len()
    }

    /// Seek to a position
    pub fn seek(&mut self, offset: usize) {
        self.position = offset.min(self.data.len());
    }

    /// Get a reference to the entire file data
    pub fn get_buffer(&self) -> &[u8] {
        &self.data
    }
}

/// Multi-archive set for reading files from multiple MPQ archives
pub struct ArchiveSet {
    archives: Vec<Archive>,
    paths: Vec<PathBuf>,
}

impl ArchiveSet {
    /// Create a new empty archive set
    pub fn new() -> Self {
        Self {
            archives: Vec::new(),
            paths: Vec::new(),
        }
    }

    /// Add an archive to the set
    pub fn add_archive(&mut self, path: &Path) -> Result<()> {
        // Validate file exists
        if !path.exists() {
            anyhow::bail!("MPQ file does not exist: {}", path.display());
        }

        // Open MPQ archive using wow-mpq
        let archive = Archive::open(path)
            .with_context(|| format!("Failed to open MPQ archive: {}", path.display()))?;

        self.archives.push(archive);
        self.paths.push(path.to_path_buf());
        Ok(())
    }

    /// Check if any archives are loaded
    pub fn is_empty(&self) -> bool {
        self.archives.is_empty()
    }

    /// Read a file from the archive set (searches in order)
    pub fn read_file(&self, file_path: &str) -> Result<Vec<u8>> {
        // Try each archive in reverse order (later archives override earlier ones)
        // We need mutable access, so we'll use interior mutability via RefCell or just make this mut
        // For now, let's use a different approach: open archives fresh each time
        for path in self.paths.iter().rev() {
            if let Ok(mut archive) = Archive::open(path) {
                if let Ok(data) = archive.read_file(file_path) {
                    return Ok(data);
                }
            }
        }

        anyhow::bail!("File not found in any archive: {}", file_path)
    }

    /// Check if a file exists in any archive
    pub fn has_file(&self, file_path: &str) -> bool {
        // Try to read the file - if successful, it exists
        self.read_file(file_path).is_ok()
    }
}

impl Default for ArchiveSet {
    fn default() -> Self {
        Self::new()
    }
}
