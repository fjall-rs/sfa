// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    toc::{
        Toc,
        entry::{SectionName, TocEntry},
        writer::TocWriter,
    },
    trailer::writer::TrailerWriter,
};
use std::io::{Seek, Write};

/// Archive writer
#[allow(clippy::struct_field_names)]
pub struct Writer<W: Write + Seek> {
    writer: W,
    last_section_pos: u64,
    section_name: SectionName,
    toc: Toc,
}

impl<W: Write + Seek> Writer<W> {
    /// Returns a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Creates a new writer with the given I/O writer.
    #[must_use]
    pub fn from_writer(writer: W) -> Self {
        Self {
            writer,
            last_section_pos: 0,
            section_name: SectionName::new(),
            toc: Toc::default(),
        }
    }
}

impl<W: Write + Seek> std::io::Write for Writer<W> {
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }
}

impl<W: Write + Seek> Writer<W> {
    /// Starts the first named section.
    ///
    /// # Errors
    ///
    /// Returns error, if an IO error occurred.
    pub fn start(&mut self, name: impl Into<SectionName>) -> std::io::Result<()> {
        self.append_toc_entry()?;
        self.section_name = name.into();
        Ok(())
    }

    fn append_toc_entry(&mut self) -> std::io::Result<()> {
        let file_pos = self.writer.stream_position()?;

        if file_pos > 0 {
            let name = std::mem::take(&mut self.section_name);
            self.toc.push(TocEntry {
                name,
                pos: self.last_section_pos,
                len: file_pos - self.last_section_pos,
            });
        }

        self.last_section_pos = file_pos;

        Ok(())
    }

    fn append_trailer(&mut self) -> crate::Result<()> {
        // Write ToC
        let toc_pos = self.writer.stream_position()?;
        let toc_checksum = TocWriter::write_into(&mut self.writer, self.toc.as_ref())?;

        let after_toc_pos = self.writer.stream_position()?;
        let toc_len = after_toc_pos - toc_pos;

        // Write trailer
        TrailerWriter::write_into(&mut self.writer, toc_checksum, toc_pos, toc_len)
    }

    /// Ensure that the table of contents is sorted by section name, returning whether the table was already sorted.
    ///
    /// For larger tables, sorting may improve lookup performance.
    pub fn sort_toc(&mut self) -> bool {
        log::trace!("Sorting table of contents");
        self.toc.sort_by_name()
    }

    /// Finishes the file.
    ///
    /// # Errors
    ///
    /// Returns error, if an IO error occurred.
    #[allow(clippy::missing_panics_doc)]
    pub fn finish(self) -> crate::Result<()> {
        self.into_inner().map(|_| ())
    }

    /// Finishes the file.
    ///
    /// Returns the inner writer.
    ///
    /// # Errors
    ///
    /// Returns error, if an IO error occurred.
    pub fn into_inner(mut self) -> crate::Result<W> {
        log::trace!("Finishing archive");

        self.append_toc_entry()?;
        self.append_trailer()?;
        self.writer.flush()?;

        Ok(self.writer)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::toc::reader::TocReader;
    use crate::trailer::reader::TrailerReader;
    use std::fs::File;
    use std::io::Write;
    use test_log::test;

    #[test]
    fn writer_empty() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("file.sfa");

        let mut file = File::create(&path)?;
        let writer = Writer::from_writer(&mut file);
        writer.finish()?;
        file.sync_all()?;
        drop(file);

        let mut reader = File::open(&path)?;
        let trailer = TrailerReader::from_reader(&mut reader)?;
        assert_eq!(0, trailer.toc_pos);

        let toc = TocReader::from_reader(&mut reader, trailer.toc_pos, trailer.toc_checksum)?;
        assert_eq!(0, toc.len());
        assert!(toc.is_empty());
        assert!(toc.section(b"hello").is_none());

        Ok(())
    }

    #[test]
    fn writer_simple() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("file.sfa");

        let data = b"hello world";

        let mut file = File::create(&path)?;
        let mut writer = Writer::from_writer(&mut file);
        writer.write_all(data)?;
        writer.finish()?;
        file.sync_all()?;
        drop(file);

        let mut reader = File::open(&path)?;
        let trailer = TrailerReader::from_reader(&mut reader)?;
        assert_eq!(data.len() as u64, trailer.toc_pos);

        let toc = TocReader::from_reader(&mut reader, trailer.toc_pos, trailer.toc_checksum)?;
        assert_eq!(1, toc.len());
        assert!(toc.section(b"hello").is_none());
        assert!(toc.section(b"").is_some());

        assert_eq!(0, toc[0].pos);
        assert_eq!(data.len() as u64, toc[0].len);
        assert_eq!(&[] as &[u8], &*toc[0].name);

        Ok(())
    }

    #[test]
    fn writer_multiple_sections() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("file.sfa");

        let data = b"hello world";
        let data2 = b"hello world2";
        let data3 = b"hello world3";

        let mut file = File::create(&path)?;
        let mut writer = Writer::from_writer(&mut file);
        writer.write_all(data)?;
        writer.start("section1")?;
        writer.write_all(data2)?;
        writer.start("section2")?;
        writer.write_all(data3)?;
        writer.finish()?;
        file.sync_all()?;
        drop(file);

        let mut reader = File::open(&path)?;
        let trailer = TrailerReader::from_reader(&mut reader)?;
        assert_eq!(
            data.len() as u64 + data2.len() as u64 + data3.len() as u64,
            trailer.toc_pos,
        );

        let toc = TocReader::from_reader(&mut reader, trailer.toc_pos, trailer.toc_checksum)?;
        assert_eq!(3, toc.len());
        assert!(toc.section(b"hello").is_none());
        assert!(toc.section(b"").is_some());
        assert!(toc.section(b"section1").is_some());
        assert!(toc.section(b"section2").is_some());

        assert_eq!(0, toc[0].pos);
        assert_eq!(data.len() as u64, toc[0].len());

        assert_eq!(&[] as &[u8], &*toc[0].name);
        assert_eq!(b"section1", &*toc[1].name);
        assert_eq!(b"section2", &*toc[2].name);

        assert_eq!(data.len() as u64, toc[0].len);
        assert_eq!(data2.len() as u64, toc[1].len);
        assert_eq!(data3.len() as u64, toc[2].len);

        Ok(())
    }
}
