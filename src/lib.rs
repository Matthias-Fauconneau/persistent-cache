#![feature(unboxed_closures,fn_traits,try_trait)]
#![allow(non_upper_case_globals)]

pub fn key<Args:std::hash::Hash>(args: &Args) -> String {
    use std::hash::Hasher;
    let mut s = std::collections::hash_map::DefaultHasher::new();
    args.hash(&mut s);
    format!("{:?}", s.finish())
}

use {anyhow::{Error, Result}, std::ops::Try};

pub trait PersistentCache {
    fn get(&mut self, _: &str) -> Result<Vec<u8>>;
    fn set(&mut self, _: &str, _: &[u8]) -> Result<()>;
    fn remove(&mut self, _: &str) -> Result<()> { anyhow::bail!("unimplemented"); }
    //fn flush(&mut self) -> Result<()>;

    fn cache<Args:std::hash::Hash,F:Fn<Args>>(&mut self, function: F, args: Args) -> Result<<F::Output as Try>::Ok> where F::Output:Try, <F::Output as Try>::Ok: serde::Serialize+serde::de::DeserializeOwned, Error:From<<F::Output as Try>::Error>, <F::Output as Try>::Error:ToString {
        let key = key(&args);
        let result: Vec<u8> = self.get(&key).unwrap();
        match result.len() {
            0 => {
                let result = function.call(args).into_result();
                self.set(&key, &bincode::serialize::<Result<&<F::Output as Try>::Ok, String>>(&result.as_ref().map_err(std::string::ToString::to_string)).unwrap()).unwrap();
                Ok(result?)
            },
            _ => bincode::deserialize::<Result<<F::Output as Try>::Ok, String>>(&result).unwrap().map_err(Error::msg),
        }
    }
}

mod file; pub use file::FileStorage;
mod file_memory; pub use file_memory::FileMemoryStorage;
mod redis; pub use self::redis::RedisStorage;

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
