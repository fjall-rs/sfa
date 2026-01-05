// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use super::writer::TOC_MAGIC;
use crate::{
    checksum::Checksum,
    toc::{entry::TocEntry, Toc},
    Result,
};
use byteorder::ReadBytesExt;
use std::io::{Read, Seek, SeekFrom};

struct ChecksummedReader<R: std::io::Read> {
    inner: R,
    hasher: xxhash_rust::xxh3::Xxh3Default,
}

impl<R: std::io::Read> ChecksummedReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            inner: reader,
            hasher: xxhash_rust::xxh3::Xxh3Default::new(),
        }
    }

    pub fn checksum(&self) -> Checksum {
        Checksum::from_raw(self.hasher.digest128())
    }
}

impl<R: std::io::Read> std::io::Read for ChecksummedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;

        #[allow(clippy::indexing_slicing)]
        self.hasher.update(&buf[..n]);

        Ok(n)
    }
}

pub struct TocReader;

impl TocReader {
    pub fn from_reader<R: Read + Seek>(
        reader: &mut R,
        toc_pos: u64,
        toc_checksum: Checksum,
    ) -> Result<Toc> {
        use byteorder::LE;

        log::trace!("Reading ToC");

        reader.seek(SeekFrom::Start(toc_pos))?;

        let mut reader = ChecksummedReader::new(reader);

        {
            let mut buf = [0u8; TOC_MAGIC.len()];
            reader.read_exact(&mut buf)?;

            if buf != TOC_MAGIC {
                log::error!("Invalid version");
                return Err(crate::Error::InvalidVersion);
            }
        }

        let len = reader.read_u32::<LE>()?;

        let mut toc = Toc::with_capacity(len as usize);
        for _ in 0..len {
            toc.push(TocEntry::read_from_file(&mut reader)?);
        }

        reader.checksum().check(toc_checksum)?;

        Ok(toc)
    }
}
