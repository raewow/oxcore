//! FFI bindings to libmpq C library
//! This provides direct access to the same MPQ library used by the C++ extractor

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use anyhow::{Context, Result};

// libmpq function signatures
#[repr(C)]
pub struct mpq_archive_s {
    _private: [u8; 0],
}

pub type mpq_archive = mpq_archive_s;
pub type libmpq__off_t = i64;

#[link(name = "mpq")]
extern "C" {
    fn libmpq__version() -> *const c_char;
    fn libmpq__archive_open(
        mpq_a: *mut *mut mpq_archive,
        filename: *const c_char,
        mode: c_int,
    ) -> c_int;

    fn libmpq__archive_close(mpq_a: *mut mpq_archive) -> c_int;

    fn libmpq__file_number(
        mpq_a: *mut mpq_archive,
        filename: *const c_char,
        filenum: *mut u32,
    ) -> c_int;

    fn libmpq__file_size_unpacked(
        mpq_a: *mut mpq_archive,
        filenum: u32,
        size: *mut libmpq__off_t,
    ) -> c_int;

    fn libmpq__file_read(
        mpq_a: *mut mpq_archive,
        filenum: u32,
        buffer: *mut u8,
        size: libmpq__off_t,
        transferred: *mut libmpq__off_t,
    ) -> c_int;

    fn libmpq__strerror(error: c_int) -> *const c_char;
}

/// MPQ Archive using libmpq FFI
pub struct LibMpqArchive {
    archive: *mut mpq_archive,
    path: String,
}

unsafe impl Send for LibMpqArchive {}

impl LibMpqArchive {
    /// Open an MPQ archive using libmpq
    pub fn open(path: &Path) -> Result<Self> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?;
        let c_path = CString::new(path_str)
            .with_context(|| format!("Failed to convert path to C string: {}", path_str))?;

        let mut archive_ptr: *mut mpq_archive = std::ptr::null_mut();

        let result = unsafe { libmpq__archive_open(&mut archive_ptr, c_path.as_ptr(), -1) };

        if result != 0 {
            let error_msg = unsafe {
                let error_str = libmpq__strerror(result);
                if error_str.is_null() {
                    format!("Unknown error {}", result)
                } else {
                    CStr::from_ptr(error_str).to_string_lossy().to_string()
                }
            };
            anyhow::bail!("Failed to open MPQ archive {}: {}", path_str, error_msg);
        }

        if archive_ptr.is_null() {
            anyhow::bail!("Failed to open MPQ archive {}: null pointer returned", path_str);
        }

        Ok(Self {
            archive: archive_ptr,
            path: path_str.to_string(),
        })
    }

    /// Read a file from the archive
    pub fn read_file(&self, filename: &str) -> Result<Option<Vec<u8>>> {
        let c_filename = CString::new(filename)
            .with_context(|| format!("Failed to convert filename to C string: {}", filename))?;

        let mut filenum: u32 = 0;
        let result = unsafe {
            libmpq__file_number(self.archive, c_filename.as_ptr(), &mut filenum)
        };

        if result != 0 {
            // File not found in this archive
            return Ok(None);
        }

        // Get file size
        let mut size: libmpq__off_t = 0;
        let result = unsafe {
            libmpq__file_size_unpacked(self.archive, filenum, &mut size)
        };

        if result != 0 {
            anyhow::bail!("Failed to get file size for {}: {}", filename, result);
        }

        if size <= 0 {
            return Ok(None);
        }

        // Read file data
        let mut buffer = vec![0u8; size as usize];
        let mut transferred: libmpq__off_t = 0;

        let result = unsafe {
            libmpq__file_read(
                self.archive,
                filenum,
                buffer.as_mut_ptr(),
                size,
                &mut transferred,
            )
        };

        if result != 0 {
            anyhow::bail!("Failed to read file {}: {}", filename, result);
        }

        if transferred != size {
            anyhow::bail!(
                "File read incomplete: expected {}, got {}",
                size,
                transferred
            );
        }

        Ok(Some(buffer))
    }

    /// List files in archive (reads (listfile))
    pub fn list_files(&self) -> Result<Vec<String>> {
        match self.read_file("(listfile)")? {
            Some(data) => {
                let listfile_str = String::from_utf8_lossy(&data);
                let files: Vec<String> = listfile_str
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
                Ok(files)
            }
            None => Ok(Vec::new()),
        }
    }
}

impl Drop for LibMpqArchive {
    fn drop(&mut self) {
        if !self.archive.is_null() {
            unsafe {
                libmpq__archive_close(self.archive);
            }
        }
    }
}

