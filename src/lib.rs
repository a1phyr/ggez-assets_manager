//! This crate provide integration of [`assets_manager`] for [`ggez`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod assets;
mod source;

pub use ::assets_manager::{AssetCache, ReloadWatcher};
pub use source::FileSystem;

/// An `AssetCache` for use with `ggez`.
pub type GgezAssetCache = assets_manager::AssetCache<FileSystem>;

/// Re-export of `assets_manager`
pub mod assets_manager {
    pub use assets_manager::*;
}

mod seal {
    pub trait Sealed {}

    impl<S: assets_manager::source::Source + ?Sized> Sealed for assets_manager::AssetCache<S> {}
}

/// Creates a new `GgezAssetCache`.
///
/// `game_id` and `author` parameters should be the same as thoses given to
/// [`ggez::ContextBuilder::new`].
pub fn new_asset_cache(game_id: &str, author: &str) -> GgezAssetCache {
    AssetCache::with_source(FileSystem::new(game_id, author))
}

/// Types that can be used with [`AssetCacheExt`].
///
/// This trait cannot be implemented outside this crate.
pub trait GgezAsset: assets::GgezAsset {}

impl GgezAsset for ggez::audio::SoundData {}
impl GgezAsset for ggez::audio::Source {}
impl GgezAsset for ggez::audio::SpatialSource {}
impl GgezAsset for ggez::graphics::Font {}
impl GgezAsset for ggez::graphics::Image {}

/// An extension trait for `AssetCache`.
///
/// This enables to easily use types for `ggez`.
pub trait AssetCacheExt: seal::Sealed {
    /// Gets an asset from the cache, and loads it from the source (usually the
    /// filesystem) if it was not found.
    fn ggez_load<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset;

    /// Gets an asset from the cache and returns an errors if it was not found.
    fn ggez_get_cached<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset;

    /// Returns `true` if an asset is present in the cache.
    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset;

    /// Returns a `ReloadWatcher` to watch changes of an asset.
    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset;
}

impl<S: assets_manager::source::Source + ?Sized> AssetCacheExt for AssetCache<S> {
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
