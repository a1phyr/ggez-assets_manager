use assets_manager::{
    asset::{NotHotReloaded, Storable},
    loader, AnyCache, Asset, BoxedError, Compound, ReloadWatcher,
};
use parking_lot::Mutex;
use std::{borrow::Cow, io};

#[cold]
fn convert_error(err: assets_manager::Error) -> ggez::GameError {
    match err.reason().downcast_ref::<io::Error>() {
        Some(io_err) if io_err.kind() == io::ErrorKind::NotFound => {
            ggez::GameError::ResourceNotFound(err.id().to_string(), Vec::new())
        }
        _ => ggez::GameError::ResourceLoadError(format!("\"{}\": {:?}", err.id(), err.reason())),
    }
}

#[cold]
fn not_found_error() -> ggez::GameError {
    ggez::GameError::CustomError(String::from("resource not found in cache"))
}

#[derive(Debug, Clone, Copy)]
struct GgezValue<T>(T);

impl<T: Send + Sync + 'static> Storable for GgezValue<T> {}

fn default_load_fast<T: GgezAsset + Clone>(
    cache: AnyCache,
    context: &mut ggez::Context,
    id: &str,
) -> ggez::GameResult<T> {
    if let Some(handle) = cache.get_cached::<GgezValue<T>>(id) {
        return Ok(handle.cloned().0);
    }

    let repr = cache
        .load_owned::<T::AssetRepr>(id)
        .map_err(convert_error)?;
    let this = T::from_owned_repr(context, repr)?;
    Ok(cache.get_or_insert(id, GgezValue(this)).cloned().0)
}

fn default_get_cached_fast<T: GgezAsset + Clone>(
    cache: AnyCache,
    _context: &mut ggez::Context,
    id: &str,
) -> ggez::GameResult<T> {
    let handle = cache
        .get_cached::<GgezValue<T>>(id)
        .ok_or_else(not_found_error)?;
    Ok(handle.cloned().0)
}

fn default_contains_fast<T: GgezAsset + Clone>(cache: AnyCache, id: &str) -> bool {
    cache.contains::<GgezValue<T>>(id)
}

#[derive(Clone)]
pub enum NoAsset {}

impl loader::Loader<NoAsset> for GgezLoader {
    fn load(_content: Cow<[u8]>, _ext: &str) -> Result<NoAsset, BoxedError> {
        unreachable!()
    }
}

impl Asset for NoAsset {
    const EXTENSIONS: &'static [&'static str] = &[];
    type Loader = GgezLoader;
}

pub trait GgezAsset: Send + Sync + Sized + 'static {
    type AssetRepr: Asset;

    fn from_repr(context: &mut ggez::Context, repr: &Self::AssetRepr) -> ggez::GameResult<Self>;

    fn from_owned_repr(
        context: &mut ggez::Context,
        repr: Self::AssetRepr,
    ) -> ggez::GameResult<Self> {
        Self::from_repr(context, &repr)
    }

    fn load(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let repr = cache.load::<Self::AssetRepr>(id).map_err(convert_error)?;
        Self::from_repr(context, &repr.read())
    }

    fn load_fast(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        Self::load(cache, context, id)
    }

    fn get_cached(
        cache: AnyCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        let repr = cache
            .get_cached::<Self::AssetRepr>(id)
            .ok_or_else(not_found_error)?;
        Self::from_repr(context, &repr.read())
    }

    fn get_cached_fast(
        cache: AnyCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        Self::get_cached(cache, context, id)
    }

    fn contains(cache: AnyCache, id: &str) -> bool {
        cache.contains::<Self::AssetRepr>(id)
    }

    fn contains_fast(cache: AnyCache, id: &str) -> bool {
        cache.contains::<Self::AssetRepr>(id)
    }

    fn reload_watcher<'a>(cache: AnyCache<'a>, id: &str) -> Option<ReloadWatcher<'a>> {
        let repr = cache.get_cached::<Self::AssetRepr>(id)?;
        Some(repr.reload_watcher())
    }
}

#[non_exhaustive]
pub struct GgezLoader;

struct GgezMutexValue<T>(Mutex<T>);

impl<T: Clone> GgezMutexValue<T> {
    fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }

    fn set(&self, value: T) {
        *self.0.lock() = value;
    }

    fn get_clone(&self) -> T {
        self.0.lock().clone()
    }
}

impl<T: Send + 'static> Storable for GgezMutexValue<T> {}
impl<T: Send + 'static> NotHotReloaded for GgezMutexValue<T> {}

struct GgezAssetRepr<T>(Mutex<Option<T>>);

impl<T: Asset> Compound for GgezAssetRepr<T> {
    fn load(cache: AnyCache, id: &assets_manager::SharedString) -> Result<Self, BoxedError> {
        let asset = cache.load_owned::<T>(id)?;
        Ok(Self(Mutex::new(Some(asset))))
    }
}

trait NewWithGgezContext: Clone + Send + Sync + 'static {
    type Asset: Asset;

    fn create(context: &mut ggez::Context, asset: Self::Asset) -> ggez::GameResult<Self>;

    fn load_with_handle(
        cache: AnyCache,
        asset_handle: assets_manager::Handle<GgezAssetRepr<Self::Asset>>,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        let asset_handle = asset_handle.read();
        let mut asset_lock = asset_handle.0.lock();
        let self_handle = cache.get_cached::<GgezMutexValue<Self>>(id);

        let handle = match (asset_lock.take(), self_handle) {
            // We are fine, go on with cached value
            (None, Some(this_handle)) => this_handle,

            // The mutex was not empty, so the value needs reloading
            (Some(asset), Some(this_handle)) => {
                match Self::create(context, asset) {
                    Ok(this) => this_handle.get().set(this.clone()),
                    Err(err) => log::error!("Failed to reload \"{id}\": {err}"),
                }
                this_handle
            }

            // The value needs reloading, but it was not previously stored.
            // This actually is the base case
            (Some(asset), None) => {
                let this = Self::create(context, asset)?;
                cache.get_or_insert(id, GgezMutexValue::new(this))
            }

            // This can happen if `Self::create` previously returned an error
            (None, None) => return Err(ggez::GameError::ResourceLoadError(id.to_string())),
        };

        Ok(handle.get().get_clone())
    }
}

impl<T: NewWithGgezContext> GgezAsset for T {
    type AssetRepr = NoAsset;

    fn from_repr(_context: &mut ggez::Context, repr: &Self::AssetRepr) -> ggez::GameResult<Self> {
        match *repr {}
    }

    fn load(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let asset_handle = cache
            .load::<GgezAssetRepr<T::Asset>>(id)
            .map_err(convert_error)?;

        Self::load_with_handle(cache, asset_handle, context, id)
    }

    fn load_fast(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        if let Some(this) = cache.get_cached::<GgezValue<Self>>(id) {
            return Ok(this.cloned().0);
        }

        let asset = cache.load_owned(id).map_err(convert_error)?;
        let this = Self::create(context, asset)?;
        let this = cache.get_or_insert(id, GgezValue(this));
        Ok(this.cloned().0)
    }

    fn get_cached(
        cache: AnyCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        let asset_handle = cache
            .get_cached::<GgezAssetRepr<T::Asset>>(id)
            .ok_or_else(not_found_error)?;

        Self::load_with_handle(cache, asset_handle, context, id)
    }

    fn get_cached_fast(
        cache: AnyCache,
        _context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        match cache.get_cached::<GgezValue<Self>>(id) {
            Some(handle) => Ok(handle.cloned().0),
            None => Err(not_found_error()),
        }
    }

    fn contains(cache: AnyCache, id: &str) -> bool {
        cache.contains::<GgezAssetRepr<T::Asset>>(id)
    }

    fn contains_fast(cache: AnyCache, id: &str) -> bool {
        cache.contains::<GgezValue<Self>>(id)
    }

    fn reload_watcher<'a>(cache: AnyCache<'a>, id: &str) -> Option<ReloadWatcher<'a>> {
        let repr = cache.get_cached::<GgezAssetRepr<T::Asset>>(id)?;
        Some(repr.reload_watcher())
    }
}

pub struct ImageAsset(image::RgbaImage);

impl Asset for ImageAsset {
    type Loader = GgezLoader;
    const EXTENSIONS: &'static [&'static str] = &["png", "bmp", "wepb"];
}

impl loader::Loader<ImageAsset> for GgezLoader {
    fn load(content: Cow<[u8]>, ext: &str) -> Result<ImageAsset, BoxedError> {
        let img: image::DynamicImage = loader::ImageLoader::load(content, ext)?;
        let img = img.to_rgba8();

        Ok(ImageAsset(img))
    }
}

impl NewWithGgezContext for ggez::graphics::Image {
    type Asset = ImageAsset;

    fn create(context: &mut ggez::Context, image: ImageAsset) -> ggez::GameResult<Self> {
        Ok(ggez::graphics::Image::from_pixels(
            context,
            &image.0,
            ggez::graphics::ImageFormat::Rgba8UnormSrgb,
            image.0.width(),
            image.0.height(),
        ))
    }
}

pub struct ShaderAsset(String);

impl From<String> for ShaderAsset {
    fn from(code: String) -> Self {
        ShaderAsset(code)
    }
}

impl Asset for ShaderAsset {
    type Loader = loader::LoadFrom<String, loader::StringLoader>;
    const EXTENSION: &'static str = "wgsl";
}

impl NewWithGgezContext for ggez::graphics::Shader {
    type Asset = ShaderAsset;

    fn create(context: &mut ggez::Context, asset: Self::Asset) -> ggez::GameResult<Self> {
        ggez::graphics::ShaderBuilder::new_wgsl()
            .combined_code(&asset.0)
            .build(context)
    }
}

pub struct FontAsset(Mutex<Option<ggez::graphics::FontData>>);

impl Asset for FontAsset {
    type Loader = GgezLoader;
    const EXTENSIONS: &'static [&'static str] = &["ttf", "otf"];
}

impl loader::Loader<FontAsset> for GgezLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<FontAsset, BoxedError> {
        let font = ggez::graphics::FontData::from_vec(content.into_owned())?;
        Ok(FontAsset(Mutex::new(Some(font))))
    }
}
pub fn set_font(
    cache: AnyCache,
    context: &mut ggez::Context,
    name: &str,
    id: &str,
) -> ggez::GameResult<()> {
    let font = cache.load::<FontAsset>(id).map_err(convert_error)?;

    if let Some(font) = font.read().0.lock().take() {
        log::debug!("Adding new font to ggez");
        context.gfx.add_font(name, font);
    }

    Ok(())
}

#[derive(Clone)]
pub struct AudioAsset(ggez::audio::SoundData);

impl Asset for AudioAsset {
    type Loader = GgezLoader;
    const EXTENSIONS: &'static [&'static str] = &["ogg", "flac", "wav"];
}

impl loader::Loader<AudioAsset> for GgezLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<AudioAsset, BoxedError> {
        Ok(AudioAsset(ggez::audio::SoundData::from_bytes(&content)))
    }
}

impl GgezAsset for ggez::audio::SoundData {
    type AssetRepr = AudioAsset;

    fn from_repr(_context: &mut ggez::Context, sound: &AudioAsset) -> ggez::GameResult<Self> {
        Ok(sound.0.clone())
    }

    fn from_owned_repr(_context: &mut ggez::Context, sound: AudioAsset) -> ggez::GameResult<Self> {
        Ok(sound.0)
    }

    fn load_fast(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        default_load_fast(cache, context, id)
    }

    fn get_cached_fast(
        cache: AnyCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        default_get_cached_fast(cache, context, id)
    }

    fn contains_fast(cache: AnyCache, id: &str) -> bool {
        default_contains_fast::<Self>(cache, id)
    }
}

impl GgezAsset for ggez::audio::Source {
    type AssetRepr = AudioAsset;

    fn from_repr(context: &mut ggez::Context, sound: &AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0.clone())
    }

    fn from_owned_repr(context: &mut ggez::Context, sound: AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0)
    }
}

impl GgezAsset for ggez::audio::SpatialSource {
    type AssetRepr = AudioAsset;

    fn from_repr(context: &mut ggez::Context, sound: &AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0.clone())
    }

    fn from_owned_repr(context: &mut ggez::Context, sound: AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0)
    }
}
