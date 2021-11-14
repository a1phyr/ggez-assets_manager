#![forbid(unsafe_code)]

pub mod assets;
pub mod source;

use assets::GgezAsset;

pub use assets_manager::{Asset, Compound, DirHandle, Handle, ReloadWatcher};
use std::io;

pub type AssetCache = assets_manager::AssetCache<source::FileSystem>;

mod seal {
    pub trait Sealed {}
    impl Sealed for super::AssetCache {}
}

pub trait AssetCacheExt: seal::Sealed + Sized {
    fn new(game_id: &str, author: &str) -> io::Result<Self>;

    fn ggez_load<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset;

    fn ggez_get_cached<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset;

    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset;

    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset;
}

impl AssetCacheExt for AssetCache {
    fn new(game_id: &str, author: &str) -> io::Result<Self> {
        let fs = source::FileSystem::new(game_id, author)?;
        Ok(Self::with_source(fs))
    }

    fn ggez_load<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::load(self, context, id)
        } else {
            T::load_fast(self, context, id)
        }
    }

    fn ggez_get_cached<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::get_cached(self, context, id)
        } else {
            T::get_cached_fast(self, context, id)
        }
    }

    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::contains(self, id)
        } else {
            T::contains_fast(self, id)
        }
    }

    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::reload_watcher(self, id)
        } else {
            self.ggez_contains::<T>(id).then(ReloadWatcher::default)
        }
    }
}
