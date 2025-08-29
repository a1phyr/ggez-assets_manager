use assets_manager::{AssetCache, BoxedError, FileAsset, OnceInitCell, ReloadWatcher};
use std::{borrow::Cow, io, sync::Mutex};

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

fn default_load_fast<T: GgezAsset + Clone>(
    cache: &AssetCache,
    context: &mut ggez::Context,
    id: &str,
) -> ggez::GameResult<T> {
    if let Some(handle) = cache.get::<GgezValue<T>>(id) {
        return Ok(handle.cloned().0);
    }

    let repr = cache
        .load_owned::<T::AssetRepr>(id)
        .map_err(convert_error)?;
    let this = T::from_owned_repr(context, repr)?;
    Ok(cache.get_or_insert(id, GgezValue(this)).cloned().0)
}

fn default_get_fast<T: GgezAsset + Clone>(
    cache: &AssetCache,
    _context: &mut ggez::Context,
    id: &str,
) -> ggez::GameResult<T> {
    let handle = cache.get::<GgezValue<T>>(id).ok_or_else(not_found_error)?;
    Ok(handle.cloned().0)
}

fn default_contains_fast<T: GgezAsset + Clone>(cache: &AssetCache, id: &str) -> bool {
    cache.contains::<GgezValue<T>>(id)
}

#[derive(Clone)]
pub enum NoAsset {}

impl FileAsset for NoAsset {
    const EXTENSIONS: &'static [&'static str] = &[];

    fn from_bytes(_: Cow<[u8]>) -> Result<Self, BoxedError> {
        unreachable!()
    }
}

pub trait GgezAsset: Send + Sync + Sized + 'static {
    type AssetRepr: FileAsset;

    fn from_repr(context: &mut ggez::Context, repr: &Self::AssetRepr) -> ggez::GameResult<Self>;

    fn from_owned_repr(
        context: &mut ggez::Context,
        repr: Self::AssetRepr,
    ) -> ggez::GameResult<Self> {
        Self::from_repr(context, &repr)
    }

    fn load(cache: &AssetCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let repr = cache.load::<Self::AssetRepr>(id).map_err(convert_error)?;
        Self::from_repr(context, &repr.read())
    }

    fn load_fast(
        cache: &AssetCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        Self::load(cache, context, id)
    }

    fn get(cache: &AssetCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let repr = cache
            .get::<Self::AssetRepr>(id)
            .ok_or_else(not_found_error)?;
        Self::from_repr(context, &repr.read())
    }

    fn get_fast(
        cache: &AssetCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        Self::get(cache, context, id)
    }

    fn contains(cache: &AssetCache, id: &str) -> bool {
        cache.contains::<Self::AssetRepr>(id)
    }

    fn contains_fast(cache: &AssetCache, id: &str) -> bool {
        cache.contains::<Self::AssetRepr>(id)
    }

    fn reload_watcher<'a>(cache: &'a AssetCache, id: &str) -> Option<ReloadWatcher<'a>> {
        let repr = cache.get::<Self::AssetRepr>(id)?;
        Some(repr.reload_watcher())
    }
}

type GgezAssetRepr<T> = OnceInitCell<<T as NewWithGgezContext>::Asset, T>;

trait NewWithGgezContext: Clone + Send + Sync + 'static {
    type Asset: FileAsset;

    fn create(context: &mut ggez::Context, asset: &Self::Asset) -> ggez::GameResult<Self>;

    fn load_with_handle(
        asset_handle: &assets_manager::Handle<GgezAssetRepr<Self>>,
        context: &mut ggez::Context,
    ) -> ggez::GameResult<Self> {
        let asset = asset_handle
            .read()
            .get_or_try_init(|asset| Self::create(context, asset))?
            .clone();
        Ok(asset)
    }
}

impl<T: NewWithGgezContext> GgezAsset for T {
    type AssetRepr = NoAsset;

    fn from_repr(_context: &mut ggez::Context, repr: &Self::AssetRepr) -> ggez::GameResult<Self> {
        match *repr {}
    }

    fn load(cache: &AssetCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let asset_handle = cache
            .load::<GgezAssetRepr<Self>>(id)
            .map_err(convert_error)?;

        Self::load_with_handle(asset_handle, context)
    }

    fn get(cache: &AssetCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let asset_handle = cache
            .get::<GgezAssetRepr<Self>>(id)
            .ok_or_else(not_found_error)?;

        Self::load_with_handle(asset_handle, context)
    }

    fn contains(cache: &AssetCache, id: &str) -> bool {
        cache.contains::<GgezAssetRepr<Self>>(id)
    }

    fn reload_watcher<'a>(cache: &'a AssetCache, id: &str) -> Option<ReloadWatcher<'a>> {
        let repr = cache.get::<GgezAssetRepr<Self>>(id)?;
        Some(repr.reload_watcher())
    }
}

pub struct ImageAsset(Vec<u8>);

impl FileAsset for ImageAsset {
    const EXTENSIONS: &'static [&'static str] = &["png", "bmp", "wepb", "jpeg", "jpg"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(ImageAsset(bytes.into_owned()))
    }
}

impl NewWithGgezContext for ggez::graphics::Image {
    type Asset = ImageAsset;

    fn create(context: &mut ggez::Context, image: &ImageAsset) -> ggez::GameResult<Self> {
        ggez::graphics::Image::from_bytes(context, &image.0)
    }
}

pub struct ShaderAsset(String);

impl FileAsset for ShaderAsset {
    const EXTENSION: &'static str = "wgsl";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        String::from_bytes(bytes).map(ShaderAsset)
    }
}

impl NewWithGgezContext for ggez::graphics::Shader {
    type Asset = ShaderAsset;

    fn create(context: &mut ggez::Context, shader: &ShaderAsset) -> ggez::GameResult<Self> {
        ggez::graphics::ShaderBuilder::new()
            .fragment_code(&shader.0)
            .vertex_code(&shader.0)
            .build(context)
    }
}

pub struct FontAsset(Mutex<Option<ggez::graphics::FontData>>);

impl FileAsset for FontAsset {
    const EXTENSIONS: &'static [&'static str] = &["ttf", "otf"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        let font = ggez::graphics::FontData::from_vec(bytes.into_owned())?;
        Ok(FontAsset(Mutex::new(Some(font))))
    }
}

pub fn set_font(
    cache: &AssetCache,
    context: &mut ggez::Context,
    name: &str,
    id: &str,
) -> ggez::GameResult<()> {
    let font = cache.load::<FontAsset>(id).map_err(convert_error)?;

    if let Some(font) = font.read().0.lock().unwrap().take() {
        log::debug!("Adding new font to ggez");
        context.gfx.add_font(name, font);
    }

    Ok(())
}

#[derive(Clone)]
pub struct AudioAsset(ggez::audio::SoundData);

impl FileAsset for AudioAsset {
    const EXTENSIONS: &'static [&'static str] = &["ogg", "flac", "wav"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(AudioAsset(ggez::audio::SoundData::from_bytes(&bytes)?))
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

    fn load_fast(
        cache: &AssetCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        default_load_fast(cache, context, id)
    }

    fn get_fast(
        cache: &AssetCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        default_get_fast(cache, context, id)
    }

    fn contains_fast(cache: &AssetCache, id: &str) -> bool {
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
