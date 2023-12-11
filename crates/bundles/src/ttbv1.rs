// Copyright 2023-2024 the Tectonic Project
// Licensed under the MIT License.

use crate::{FileIndex, FileInfo};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    io::{BufRead, BufReader, Read},
    str::FromStr,
};
use tectonic_errors::prelude::*;
use tectonic_io_base::digest::{self, DigestData};

pub struct TTBv1Header {
    pub index_start: u64,
    pub index_real_len: u32,
    pub index_gzip_len: u32,
    pub digest: DigestData,
}

impl TryFrom<[u8; 70]> for TTBv1Header {
    type Error = Error;

    fn try_from(header: [u8; 70]) -> Result<Self, Self::Error> {
        let signature = &header[0..14];
        let version = u64::from_le_bytes(header[14..22].try_into()?);
        let index_start = u64::from_le_bytes(header[22..30].try_into()?);
        let index_gzip_len = u32::from_le_bytes(header[30..34].try_into()?);
        let index_real_len = u32::from_le_bytes(header[34..38].try_into()?);
        let digest: DigestData = DigestData::from_str(&digest::bytes_to_hex(&header[38..70]))?;

        if signature != b"tectonicbundle" {
            bail!("this is not a bundle");
        }

        if version != 1 {
            bail!("wrong ttb version");
        }

        return Ok(TTBv1Header {
            digest,
            index_start,
            index_real_len,
            index_gzip_len,
        });
    }
}

/// file info for TTbundle
#[derive(Clone, Debug)]
pub struct TTBFileInfo {
    pub start: u64,
    pub real_len: u32,
    pub gzip_len: u32,
    pub path: String,
    pub name: String,
    pub hash: Option<String>,
}

impl FileInfo for TTBFileInfo {
    fn name(&self) -> &str {
        return &self.name;
    }
    fn path(&self) -> &str {
        return &self.path;
    }
}

pub struct TTBFileIndex {
    // Vector of fileinfos.
    // This MUST be sorted by path for search() to work properly!
    pub content: Vec<TTBFileInfo>,

    search: Vec<String>,

    // Remember previous searches so we don't have to iterate over content again.
    search_cache: HashMap<String, Option<TTBFileInfo>>,
}

impl TTBFileIndex {
    pub fn new() -> Self {
        return Self {
            content: Vec::new(),
            search: Vec::new(),
            search_cache: HashMap::new(),
        };
    }
}

impl TTBFileIndex {
    fn read_filelist_line(&mut self, line: &str) -> Result<()> {
        let mut bits = line.split_whitespace();

        if let (Some(start), Some(gzip_len), Some(real_len), Some(path), Some(hash)) = (
            bits.next(),
            bits.next(),
            bits.next(),
            bits.next(),
            bits.next(),
        ) {
            let (_, name) = path.rsplit_once("/").unwrap();

            self.content.push(TTBFileInfo {
                start: start.parse::<u64>()?,
                gzip_len: gzip_len.parse::<u32>()?,
                real_len: real_len.parse::<u32>()?,
                path: path.to_owned(),
                name: name.to_owned(),
                hash: match hash {
                    "nohash" => None,
                    _ => Some(hash.to_owned()),
                },
            });
        } else {
            // TODO: preserve the warning info or something!
            bail!("malformed FILELIST line");
        }

        return Ok(());
    }

    fn read_search_line(&mut self, line: String) -> Result<()> {
        self.search.push(line);
        return Ok(());
    }
}

impl<'this> FileIndex<'this, TTBFileInfo> for TTBFileIndex {
    fn iter(&'this self) -> Box<dyn Iterator<Item = &'this TTBFileInfo> + 'this> {
        return Box::new(self.content.iter());
    }

    fn len(&self) -> usize {
        return self.content.len();
    }

    fn initialize(&mut self, reader: &mut dyn Read) -> Result<()> {
        self.content.clear();
        self.search.clear();
        self.search_cache.clear();

        let mut mode: Option<String> = None;
        for line in BufReader::new(reader).lines() {
            let line = line?;

            if line.starts_with("[") {
                mode = Some(line[1..line.len() - 1].to_owned());
                continue;
            }

            if mode.is_none() {
                continue;
            }

            match &mode.as_ref().unwrap()[..] {
                "FILELIST" => self.read_filelist_line(&line)?,
                "SEARCH:MAIN" => self.read_search_line(line)?,
                _ => continue,
            }
        }

        return Ok(());
    }

    fn search(&'this mut self, name: &str) -> Option<TTBFileInfo> {
        match self.search_cache.get(name) {
            None => {}
            Some(r) => return r.clone(),
        }

        // Get last element of path, since
        // some packages reference a path to a file.
        // `fithesis4` is one example.
        let relative_parent: bool;

        let n = match name.rsplit_once("/") {
            Some(n) => {
                relative_parent = true;
                n.1
            }
            None => {
                relative_parent = false;
                name
            }
        };

        // If we don't have this path in the index, this file doesn't exist.
        // The code below will clone these strings iff it has to.
        let mut infos: Vec<&TTBFileInfo> = Vec::new();
        for i in self.iter() {
            if i.name() == n {
                infos.push(&i);
            } else if infos.len() != 0 {
                // infos is sorted, so we can stop searching now.
                break;
            }
        }

        if relative_parent {
            // TODO: REWORK
            let mut matching: Option<&TTBFileInfo> = None;
            for info in &infos {
                if info.path().ends_with(&name) {
                    match matching {
                        Some(_) => return None, // TODO: warning. This shouldn't happen.
                        None => matching = Some(info),
                    }
                }
            }
            let matching = Some(matching?.clone());
            self.search_cache.insert(name.to_owned(), matching.clone());
            return matching;
        } else {
            // Even if paths.len() is 1, we don't return here.
            // We need to make sure this file matches a search path:
            // if it's in a directory we don't search, we shouldn't find it!

            let mut picked: Vec<&TTBFileInfo> = Vec::new();
            for rule in &self.search {
                for info in &infos {
                    if rule.ends_with("//") {
                        // Match start of patent path
                        // (cutting off the last slash)
                        if info.path().starts_with(&rule[0..rule.len() - 1]) {
                            picked.push(info);
                        }
                    } else {
                        // Match full parent path
                        if &info.path()[0..info.path().len() - name.len()] == rule {
                            picked.push(info);
                        }
                    }
                }
                if picked.len() != 0 {
                    break;
                }
            }

            let r = {
                if picked.len() == 0 {
                    // No file in our search dirs had this name.
                    None
                } else if picked.len() == 1 {
                    // We found exactly one file with this name.
                    //
                    // This chain of functions is essentially picked[0],
                    // but takes ownership of the string without requiring
                    // a .clone().
                    Some(picked[0].clone())
                } else {
                    // We found multiple files with this name, all of which
                    // have the same priority. Pick alphabetically to emulate
                    // an "alphabetic DFS" search order.
                    picked.sort_by(|a, b| a.path().cmp(&b.path()));
                    Some(picked[0].clone())
                }
            };

            self.search_cache.insert(name.to_owned(), r.clone());
            return r;
        }
    }
}
