#![forbid(unsafe_code)]

pub mod assets;
pub mod source;

use assets::GgezAsset;

pub use assets_manager::{Asset, AssetCache, Compound, DirHandle, Handle, ReloadWatcher};
use std::io;

pub type GgezAssetCache = assets_manager::AssetCache<source::FileSystem>;

mod seal {
    pub trait Sealed {}
    impl<S: crate::source::Source + ?Sized> Sealed for crate::AssetCache<S> {}
}

pub fn new_asset_cache(game_id: &str, author: &str) -> io::Result<AssetCache<source::FileSystem>> {
    let fs = source::FileSystem::new(game_id, author)?;
    Ok(AssetCache::with_source(fs))
}

pub trait AssetCacheExt: seal::Sealed {
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

impl<S: source::Source + ?Sized> AssetCacheExt for AssetCache<S> {
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
