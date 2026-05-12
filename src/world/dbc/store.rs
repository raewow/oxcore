use crate::world::dbc::file_loader::{DbcFileLoader, DbcRecord};
use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct DbcStore<T> {
    entries: HashMap<u32, T>,
    format: String,
}

impl<T> DbcStore<T> {
    pub fn new(format: &str) -> Self {
        Self {
            entries: HashMap::new(),
            format: format.to_string(),
        }
    }

    pub fn format(&self) -> &str {
        &self.format
    }

    pub fn lookup(&self, id: u32) -> Option<&T> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, id: u32, entry: T) {
        self.entries.insert(id, entry);
    }

    pub fn entries(&self) -> impl Iterator<Item = (&u32, &T)> {
        self.entries.iter()
    }
}

pub trait DbcEntry: Sized {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>>;
}

pub fn load_dbc_store<T: DbcEntry>(store: &mut DbcStore<T>, filename: &str) -> Result<()> {
    let mut loader = DbcFileLoader::new();
    loader
        .load(filename, store.format())
        .with_context(|| format!("Failed to load DBC file: {}", filename))?;

    let record_count = loader.record_count();
    for i in 0..record_count as usize {
        if let Some(record) = loader.get_record(i) {
            if let Some((id, entry)) = T::from_record(&record)? {
                store.insert(id, entry);
            }
        }
    }

    Ok(())
}
