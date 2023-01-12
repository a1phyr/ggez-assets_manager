//! This crate provide integration of [`assets_manager`] for [`ggez`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod assets;
mod source;

pub use assets_manager;

use assets_manager::AnyCache;
pub use assets_manager::{AssetCache, ReloadWatcher};
pub use source::GgezFileSystem;

/// An `AssetCache` for use with `ggez`.
pub type GgezAssetCache = assets_manager::AssetCache<GgezFileSystem>;

mod seal {
    pub trait Sealed {}

    impl<S: assets_manager::source::Source> Sealed for assets_manager::AssetCache<S> {}
    impl<'a> Sealed for assets_manager::AnyCache<'a> {}
}

/// Creates a new `GgezAssetCache`.
///
/// `game_id` and `author` parameters should be the same as thoses given to
/// [`ggez::ContextBuilder::new`].
#[must_use]
#[deprecated = "use `create_asset_cache` instead"]
#[allow(deprecated)]
pub fn new_asset_cache(game_id: &str, author: &str) -> GgezAssetCache {
    AssetCache::with_source(GgezFileSystem::new(game_id, author))
}

/// Creates a new `GgezAssetCache`.
///
/// `game_id` and `author` parameters should be the same as thoses given to
/// [`ggez::ContextBuilder::new`].
///
/// Note that resources added via `ContextBuilder::add_resource_path` or
/// `ContextBuilder::add_zip_file` are not supported at the moment.
#[must_use]
pub fn create_asset_cache(
    fs: &impl ggez::context::Has<ggez::filesystem::Filesystem>,
) -> GgezAssetCache {
    AssetCache::with_source(GgezFileSystem::from_context(fs))
}

/// Types that can be used with [`AssetCacheExt`].
///
/// This trait cannot be implemented outside this crate.
pub trait GgezAsset: assets::GgezAsset {}

impl GgezAsset for ggez::audio::SoundData {}
impl GgezAsset for ggez::audio::Source {}
impl GgezAsset for ggez::audio::SpatialSource {}
// impl GgezAsset for ggez::graphics::Font {}
impl GgezAsset for ggez::graphics::Image {}
impl GgezAsset for ggez::graphics::Shader {}

/// An extension trait for `AssetCache`.
///
/// This enables to easily use types for `ggez`.
///
/// Note that unlike other [`AssetCache`] methods that return a [`Handle`],
/// these methods directly return the requested type.
///
/// [`Handle`]: assets_manager::Handle
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
    ///
    /// Returns `None` if the asset is not in the cache.
    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset;

    /// Add a font to `ggez` with the given name, loaded from the given id.
    fn set_font(&self, context: &mut ggez::Context, name: &str, id: &str) -> ggez::GameResult<()>;
}

impl<S: assets_manager::source::Source> AssetCacheExt for AssetCache<S> {
    fn ggez_load<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::load(self.as_any_cache(), context, id)
        } else {
            T::load_fast(self.as_any_cache(), context, id)
        }
    }

    fn ggez_get_cached<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::get_cached(self.as_any_cache(), context, id)
        } else {
            T::get_cached_fast(self.as_any_cache(), context, id)
        }
    }

    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::contains(self.as_any_cache(), id)
        } else {
            T::contains_fast(self.as_any_cache(), id)
        }
    }

    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::reload_watcher(self.as_any_cache(), id)
        } else {
            self.ggez_contains::<T>(id).then(ReloadWatcher::default)
        }
    }

    fn set_font(&self, context: &mut ggez::Context, name: &str, id: &str) -> ggez::GameResult<()> {
        assets::set_font(self.as_any_cache(), context, name, id)
    }
}

impl<'a> AssetCacheExt for AnyCache<'a> {
    fn ggez_load<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::load(*self, context, id)
        } else {
            T::load_fast(*self, context, id)
        }
    }

    fn ggez_get_cached<T>(&self, context: &mut ggez::Context, id: &str) -> ggez::GameResult<T>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::get_cached(*self, context, id)
        } else {
            T::get_cached_fast(*self, context, id)
        }
    }

    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::contains(*self, id)
        } else {
            T::contains_fast(*self, id)
        }
    }

    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher>
    where
        T: GgezAsset,
    {
        if cfg!(feature = "hot-reloading") {
            T::reload_watcher(*self, id)
        } else {
            self.ggez_contains::<T>(id).then(ReloadWatcher::default)
        }
    }

    fn set_font(&self, context: &mut ggez::Context, name: &str, id: &str) -> ggez::GameResult<()> {
        assets::set_font(*self, context, name, id)
    }
}
