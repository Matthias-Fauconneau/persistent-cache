#![feature(unboxed_closures,fn_traits,try_trait)]
#![allow(non_upper_case_globals)]

pub fn key<Args:std::hash::Hash>(args: &Args) -> String {
    use std::hash::Hasher;
    let mut s = std::collections::hash_map::DefaultHasher::new();
    args.hash(&mut s);
    format!("{:?}", s.finish())
}

use {fehler::throws, anyhow::{Error, Result}, std::ops::Try};

pub trait PersistentCache {
    #[throws(std::io::Error)] fn read(&mut self, key: &str) -> Vec<u8>;
    #[allow(redundant_semicolons)] #[throws(std::io::Error)] fn write(&mut self, _key: &str, _value: &[u8]);
    #[allow(unreachable_code)] #[throws] fn remove(&mut self, _: &str) { anyhow::bail!("unimplemented") }
    //fn flush(&mut self) -> Result<()>;

    fn cache<Args:std::hash::Hash,F:Fn<Args>>(&mut self, function: F, args: Args) -> Result<<F::Output as Try>::Ok> where F::Output:Try, <F::Output as Try>::Ok: serde::Serialize+serde::de::DeserializeOwned, Error:From<<F::Output as Try>::Error>, <F::Output as Try>::Error:ToString {
        let key = key(&args);
        match self.read(&key) {
            Ok(result) => {
                log::trace!("read {}", key);
                bincode::deserialize::<Result<<F::Output as Try>::Ok, String>>(&result)?.map_err(Error::msg)
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::trace!("evaluate {}", key);
                let result = function.call(args).into_result();
                self.write(&key, &bincode::serialize::<Result<&<F::Output as Try>::Ok, String>>(&result.as_ref().map_err(std::string::ToString::to_string))?)?;
                log::trace!("write {}", key);
                Ok(result?)
            },
            Err(e) => Err(e)?,
        }
    }
}

pub struct FileStorage { path: std::path::PathBuf }
impl FileStorage {
    #[throws] pub fn new(path: std::path::PathBuf) -> Self {
        std::fs::create_dir_all(&path)?;
        FileStorage{path}
    }
}
impl PersistentCache for FileStorage {
    #[throws(std::io::Error)] fn read(&mut self, key: &str) -> Vec<u8> { log::trace!("read {:?}", self.path.join(key)); std::fs::read(&self.path.join(key))? }
    #[throws(std::io::Error)] fn write(&mut self, key: &str, value: &[u8])  { std::fs::write(&self.path.join(key), value)? }
}

lazy_static::lazy_static!{ pub static ref tmp: std::path::PathBuf = std::env::temp_dir(); }
lazy_static::lazy_static!{ pub static ref home: std::path::PathBuf = dirs::cache_dir().unwrap(); }

#[macro_export] macro_rules! cache {
    ($storage:ident $id:ident, $(#[$attr:meta])* $v:vis fn $f:ident($($x:ident : $t:ty),*) -> $r:ty, $impl:item) => {
        mod $id {
            use super::*;
            $impl
            #[allow(non_upper_case_globals)] pub fn storage() -> impl std::ops::DerefMut<Target=impl $crate::PersistentCache> {
                type Storage = $crate::FileStorage;
                ::lazy_static::lazy_static!{ static ref $id: std::sync::Mutex<Storage> = Storage::new($crate::$storage.join(stringify!($id))).unwrap().into(); }
                $id.lock().unwrap()
            }
        }
        $(#[$attr])* $v fn $f($($x: $t),*) -> $r { use $crate::PersistentCache; $id::storage().cache(self::$id::$f, ($($x),*,)) }
    };
    ($storage:ident $id:ident, $(#[$attr:meta])* $v:vis fn $f:ident($($x:ident : $t:ty),*) -> $r:ty $b:block) => {
        $crate::cache!{$storage $id, $(#[$attr])* $v fn $f($($x: $t),*) -> $r, $(#[$attr])* pub fn $f($($x: $t),*) -> $r $b}
    };
}
