// Copyright 2023-2024 the Tectonic Project
// Licensed under the MIT License.

//! Read ttb v1 bundles on the filesystem.
//!
//! The main type offered by this module is the [`Ttbv1NetBundle`] struct.

use crate::{
    ttbv1::{TTBFileIndex, TTBFileInfo, TTBv1Header},
    Bundle, FileIndex, FileInfo,
};
use flate2::read::GzDecoder;
use std::{
    convert::TryFrom,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};
use tectonic_errors::prelude::*;
use tectonic_io_base::{digest::DigestData, InputHandle, InputOrigin, IoProvider, OpenResult};
use tectonic_status_base::StatusBackend;

/// Read a [`TTBFileInfo`] from this bundle.
/// We assume that `fileinfo` points to a valid file in this bundle.
fn read_fileinfo<'a>(fileinfo: &TTBFileInfo, reader: &'a mut File) -> Result<Box<dyn Read + 'a>> {
    reader.seek(SeekFrom::Start(fileinfo.start))?;
    return Ok(Box::new(GzDecoder::new(
        reader.take(fileinfo.gzip_len as u64),
    )));
}

/// A bundle backed by a ZIP file.
pub struct Ttbv1FsBundle<T>
where
    for<'a> T: FileIndex<'a>,
{
    file: File,
    index: T,
}

/// The internal file-information struct used by the [`Ttbv1FsBundle`].

impl Ttbv1FsBundle<TTBFileIndex> {
    /// Create a new ZIP bundle for a generic readable and seekable stream.
    pub fn new(file: File) -> Result<Self> {
        Ok(Ttbv1FsBundle {
            file,
            index: TTBFileIndex::new(),
        })
    }

    fn get_header(&mut self) -> Result<TTBv1Header> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut header: [u8; 70] = [0u8; 70];
        self.file.read_exact(&mut header)?;
        self.file.seek(SeekFrom::Start(0))?;
        let header = TTBv1Header::try_from(header)?;
        return Ok(header);
    }

    // Fill this bundle's search rules, fetching files from our backend.
    fn fill_index(&mut self) -> Result<()> {
        let header = self.get_header()?;
        let info = TTBFileInfo {
            start: header.index_start,
            gzip_len: header.index_real_len,
            real_len: header.index_gzip_len,
            path: "/INDEX".to_owned(),
            name: "INDEX".to_owned(),
            hash: None,
        };

        let mut reader = read_fileinfo(&info, &mut self.file)?;
        self.index.initialize(&mut reader)?;

        return Ok(());
    }

    /// Open a file on the filesystem as a zip bundle.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new(File::open(path)?)
    }
}

impl IoProvider for Ttbv1FsBundle<TTBFileIndex> {
    fn input_open_name(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        // Fetch index if it is empty
        if self.index.len() == 0 {
            match self.fill_index() {
                Err(e) => return OpenResult::Err(e.into()),
                _ => {}
            }
        }

        let info = match self.index.search(&name) {
            None => return OpenResult::NotAvailable,
            Some(s) => s,
        };

        let mut v: Vec<u8> = Vec::with_capacity(info.real_len as usize);

        match read_fileinfo(&info, &mut self.file) {
            Err(e) => return OpenResult::Err(e.into()),
            Ok(mut b) => match b.read_to_end(&mut v) {
                Err(e) => return OpenResult::Err(e.into()),
                Ok(_) => {}
            },
        };

        return OpenResult::Ok(InputHandle::new_read_only(
            name,
            Cursor::new(v),
            InputOrigin::Other,
        ));
    }
}

impl Bundle for Ttbv1FsBundle<TTBFileIndex> {
    fn all_files(&self) -> Vec<String> {
        self.index.iter().map(|x| x.path().to_owned()).collect()
    }

    fn get_digest(&mut self) -> Result<DigestData> {
        let header = self.get_header()?;
        return Ok(header.digest.clone());
    }
}
