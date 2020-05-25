// Copyright 2018 Stefan Kroboth
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! # FileStorage
//!
//! Storage for persistently saving return values of functions on disk.
//! This does not cache data in memory, only on disk!
use crate::errors::*;
use fs2::FileExt;
//use regex::Regex;
//use std::error::Error;
use std::fs::{create_dir_all, /*read_dir, remove_file,*/ File};
use std::io::prelude::*;
use std::path::Path;

use crate::PersistentCache;
#[allow(unused_imports)]
use crate::PREFIX;

/// `FileStorage` struct
pub struct FileStorage {
    /// Indicates where files are saved
    path: std::path::PathBuf,
}

impl FileStorage {
    /// Creates the `path` directory and returns a `FileStorage` struct.
    ///
    /// # Example
    ///
    /// ```
    /// use persistentcache::storage::file::FileStorage;
    ///
    /// let s = FileStorage::new(".example_dir").unwrap();
    /// ```
    // pub fn new(path: &'a str) -> Result<Self, Box<Error>> {
    pub fn new(path: std::path::PathBuf) -> Result<Self> {
        create_dir_all(&path)?;
        Ok(FileStorage {
            path,
        })
    }
}

impl PersistentCache for FileStorage {
    /// Returns the value corresponding to the variable `name`.
    fn get(&mut self, name: &str) -> Result<Vec<u8>> {
        let fpath = self.path.join(name);
        let p = Path::new(&fpath);
        let mut file = match File::open(&p) {
            Err(_) => return Ok(vec![]),
            Ok(f) => f,
        };
        file.lock_exclusive()?;
        let mut s: Vec<u8> = Vec::new();
        match file.read_to_end(&mut s) {
            Ok(_) => {
                file.unlock()?;
                Ok(s.to_vec())
            }
            Err(e) => {
                file.unlock()?;
                Err(e.into())
            }
        }
    }

    /// Writes the data of type `&[u8]` in array `val` to the file corresponding to the variable `name`.
    fn set(&mut self, name: &str, val: &[u8]) -> Result<()> {
        let fpath = self.path.join(name);
        let p = Path::new(&fpath);
        let mut file = match File::create(&p) {
            Err(e) => return Err(e.into()),
            Ok(f) => f,
        };

        file.lock_exclusive()?;
        file.write_all(val)?;
        file.unlock()?;
        Ok(())
    }

    /*/// Delete all variables stored in `path` (see `new()`) which start with `PREFIX_`.
    fn flush(&mut self) -> Result<()> {
        let p = Path::new(&self.path);
        match read_dir(p) {
            Err(e) => return Err(e.into()),
            Ok(iterator) => {
                let re = Regex::new(&format!(r"^{}/{}_", self.path, PREFIX))?;
                for file in iterator {
                    let tmp = file?.path();
                    let f = tmp.to_str().unwrap();
                    if re.is_match(f) {
                        remove_file(f)?
                    }
                }
            }
        }
        Ok(())
    }*/
}
