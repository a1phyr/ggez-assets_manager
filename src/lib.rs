//! This crate provides integration of [`assets_manager`] for [`ggez`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod assets;
mod source;

pub use assets::convert_error;
pub use assets_manager::{self, AssetCache};
pub use source::GgezFileSystem;

use assets_manager::{ArcHandle, Asset, AssetReadGuard, Handle, OnceInitCell};
use ggez::GameResult;

/// Assets that require a [`ggez::Context`] to be loaded.
///
/// Full initializtion is done in two steps: first the "raw" asset, then the
/// part that requires  the `ggez::Context`.
pub trait GgezAsset: Sized + Send + Sync + 'static {
    /// The raw value, that doesn't require a `ggez::Context`.
    type Raw: Asset;

    /// Converts the raw value to the the actual asset.
    fn from_raw(raw: &mut Self::Raw, ctx: &mut ggez::Context) -> GameResult<Self>;
}

/// Stores types that implement [`GgezAsset`].
pub struct GgezStorage<T: GgezAsset>(OnceInitCell<T::Raw, T>);

impl<T: GgezAsset> GgezStorage<T> {
    /// Creates a new uninitialized storage.
    pub const fn new(raw: T::Raw) -> Self {
        Self(OnceInitCell::new(raw))
    }

    /// Gets the value if it was initialized.
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }

    /// Get the value, initializing it if needed.
    pub fn get_or_init(&self, ctx: &mut ggez::Context) -> GameResult<&T> {
        self.0.get_or_try_init(|raw| T::from_raw(raw, ctx))
    }
}

impl<T: GgezAsset> Asset for GgezStorage<T> {
    fn load(
        cache: &AssetCache,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        Asset::load(cache, id).map(Self)
    }
}

impl<T: GgezAsset> assets_manager::asset::DirLoadable for GgezStorage<T>
where
    T::Raw: assets_manager::asset::DirLoadable,
{
    fn select_ids(
        cache: &AssetCache,
        id: &assets_manager::SharedString,
    ) -> std::io::Result<Vec<assets_manager::SharedString>> {
        T::Raw::select_ids(cache, id)
    }

    fn sub_directories(
        cache: &AssetCache,
        id: &assets_manager::SharedString,
        f: impl FnMut(&str),
    ) -> std::io::Result<()> {
        T::Raw::sub_directories(cache, id, f)
    }
}

/// A helper type for ggez assets handles.
pub type GgezHandle<T> = Handle<GgezStorage<T>>;
/// A helper type for ggez assets strong handles.
pub type ArcGgezHandle<T> = ArcHandle<GgezStorage<T>>;

mod seal {
    pub trait Sealed {}
    impl Sealed for assets_manager::AssetCache {}
    impl<T: crate::GgezAsset> Sealed for crate::GgezHandle<T> {}
}

/// An extension trait for [`Handle`]`<`[`GgezStorage`]`<T>>`.
pub trait HandleExt: seal::Sealed {
    /// The actual asset type.
    type Target;

    /// Locks the asset for reading and gets it if it was initialized.
    fn read_get(&self) -> Option<AssetReadGuard<'_, Self::Target>>;

    /// Locks the asset for reading, initializing it if needed.
    fn read_init(&self, ctx: &mut ggez::Context) -> GameResult<AssetReadGuard<'_, Self::Target>>;

    /// Get a clone of the asset, initializing it if needed.
    fn get_cloned(&self, ctx: &mut ggez::Context) -> GameResult<Self::Target>
    where
        Self::Target: Clone,
    {
        self.read_init(ctx).map(|g| g.clone())
    }
}

impl<T: GgezAsset> HandleExt for Handle<GgezStorage<T>> {
    type Target = T;

    fn read_get(&self) -> Option<AssetReadGuard<'_, T>> {
        AssetReadGuard::try_map(self.read(), |x| x.get()).ok()
    }

    fn read_init(&self, ctx: &mut ggez::Context) -> GameResult<AssetReadGuard<'_, T>> {
        let mut err = None;
        AssetReadGuard::try_map(self.read(), |x| match x.get_or_init(ctx) {
            Ok(x) => Some(x),
            Err(e) => {
                err = Some(e);
                None
            }
        })
        .map_err(|_| err.unwrap())
    }
}

/// Creates a new `AssetCache` backed by a [`GgezFileSystem`].
#[must_use]
pub fn new_asset_cache(fs: &impl ggez::context::Has<ggez::filesystem::Filesystem>) -> AssetCache {
    AssetCache::with_source(GgezFileSystem::from_context(fs))
}

/// An extension trait for [`AssetCache`].
///
/// This enables to easily use types that implement `GgezAsset`.
pub trait AssetCacheExt: seal::Sealed {
    /// Creates a new `AssetCache` backed by a [`GgezFileSystem`].
    fn new_ggez(ctx: &impl ggez::context::Has<ggez::filesystem::Filesystem>) -> Self;

    /// Loads a `ggez` asset.
    fn ggez_load<T>(&self, id: &str) -> GameResult<&Handle<GgezStorage<T>>>
    where
        T: GgezAsset;

    /// Loads a `ggez` asset and initialize it.
    ///
    /// The initialization is only done once per asset.
    fn ggez_load_init<T>(
        &self,
        ctx: &mut ggez::Context,
        id: &str,
    ) -> GameResult<AssetReadGuard<'_, T>>
    where
        T: GgezAsset,
    {
        self.ggez_load(id)?.read_init(ctx)
    }

    /// Loads a `ggez` asset, initialize and clone it.
    ///
    /// The initialization is only done once per asset.
    fn ggez_load_clone<T>(&self, ctx: &mut ggez::Context, id: &str) -> GameResult<T>
    where
        T: GgezAsset + Clone,
    {
        self.ggez_load(id)?.get_cloned(ctx)
    }

    /// Gets a `ggez` asset from the cache.
    fn ggez_get<T>(&self, id: &str) -> Option<&Handle<GgezStorage<T>>>
    where
        T: GgezAsset;

    /// Gets a `ggez` asset from the cache and initialize it.
    ///
    /// The initialization is only done once per asset.
    fn ggez_get_init<T>(
        &self,
        ctx: &mut ggez::Context,
        id: &str,
    ) -> GameResult<AssetReadGuard<'_, T>>
    where
        T: GgezAsset,
    {
        let not_found =
            || ggez::GameError::ResourceLoadError("resource not found in cache".to_owned());

        self.ggez_get(id).ok_or_else(not_found)?.read_init(ctx)
    }

    /// Gets a `ggez` asset from the cache, initialize and clone it.
    ///
    /// The initialization is only done once per asset.
    fn ggez_get_clone<T>(&self, ctx: &mut ggez::Context, id: &str) -> GameResult<T>
    where
        T: GgezAsset + Clone,
    {
        self.ggez_get_init::<T>(ctx, id).map(|x| x.clone())
    }

    /// Returns `true` if an asset is present in the cache.
    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset;

    /// Add a font to `ggez` with the given name, loaded from the given id.
    fn set_font(&self, context: &mut ggez::Context, name: &str, id: &str) -> GameResult<()>;
}

impl AssetCacheExt for AssetCache {
    #[inline]
    fn new_ggez(fs: &impl ggez::context::Has<ggez::filesystem::Filesystem>) -> Self {
        new_asset_cache(fs)
    }

    #[inline]
    fn ggez_load<T>(&self, id: &str) -> GameResult<&Handle<GgezStorage<T>>>
    where
        T: GgezAsset,
    {
        self.load::<GgezStorage<T>>(id)
            .map_err(crate::assets::convert_error)
    }

    #[inline]
    fn ggez_get<T>(&self, id: &str) -> Option<&Handle<GgezStorage<T>>>
    where
        T: GgezAsset,
    {
        self.get(id)
    }

    #[inline]
    fn ggez_contains<T>(&self, id: &str) -> bool
    where
        T: GgezAsset,
    {
        self.contains::<GgezStorage<T>>(id)
    }

    fn set_font(&self, ctx: &mut ggez::Context, name: &str, id: &str) -> GameResult<()> {
        assets::set_font(self, ctx, name, id)
    }
}
