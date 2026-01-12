// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::TocEntry;

pub mod entry;
pub mod reader;
pub mod writer;

const BINARY_SEARCH_THRESHOLD: usize = 64;

/// Table of contents
pub struct Toc {
    entries: Vec<TocEntry>,
    is_sorted_by_name: bool,
}

impl Default for Toc {
    fn default() -> Self {
        Self {
            entries: Vec::default(),
            is_sorted_by_name: true,
        }
    }
}

impl Toc {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            is_sorted_by_name: true,
        }
    }

    /// Find a section by name.
    #[must_use]
    pub fn section(&self, name: &[u8]) -> Option<&TocEntry> {
        if self.is_sorted_by_name && self.entries.len() > BINARY_SEARCH_THRESHOLD {
            self.entries
                .binary_search_by_key(&name, |e| e.name())
                .ok()
                .and_then(|idx| self.entries.get(idx))
        } else {
            self.iter().find(|entry| entry.name() == name)
        }
    }

    /// Add a new entry to the end of the table,
    /// tracking whether or not the table is/ remains sorted.
    pub(crate) fn push(&mut self, entry: TocEntry) {
        let empty: [u8; 0] = [];
        if self.is_sorted_by_name
            && self
                .entries
                .last()
                .map_or(empty.as_slice(), |e: &TocEntry| e.name())
                > entry.name()
        {
            self.is_sorted_by_name = false;
        }
        self.entries.push(entry);
    }

    /// Ensure that the table of contents is sorted by section name,
    /// returning whether the table was already sorted.
    ///
    /// For larger tables, sorting may improve lookup performance.
    pub fn sort_by_name(&mut self) -> bool {
        if self.is_sorted_by_name {
            return true;
        }
        self.entries.sort_by(|a, b| a.name().cmp(b.name()));
        self.is_sorted_by_name = true;
        false
    }
}

impl std::ops::Deref for Toc {
    type Target = [TocEntry];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}
