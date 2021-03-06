use assets_manager::{
    asset::{NotHotReloaded, Storable},
    loader, AnyCache, Asset, BoxedError, ReloadWatcher,
};
use parking_lot::Mutex;
use std::{borrow::Cow, io};

#[cold]
fn convert_error(err: assets_manager::Error) -> ggez::GameError {
    match err.reason().downcast_ref::<io::Error>() {
        Some(io_err) if io_err.kind() == io::ErrorKind::NotFound => {
            ggez::GameError::ResourceNotFound(err.id().to_owned(), Vec::new())
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

    let repr = cache.load_owned::<T::MidRepr>(id).map_err(convert_error)?;
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

pub trait GgezAsset: Send + Sync + Sized + 'static {
    type MidRepr: Asset;

    fn from_repr(context: &mut ggez::Context, repr: &Self::MidRepr) -> ggez::GameResult<Self>;

    fn from_owned_repr(context: &mut ggez::Context, repr: Self::MidRepr) -> ggez::GameResult<Self> {
        Self::from_repr(context, &repr)
    }

    fn load(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let repr = cache.load::<Self::MidRepr>(id).map_err(convert_error)?;
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
            .get_cached::<Self::MidRepr>(id)
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
        cache.contains::<Self::MidRepr>(id)
    }

    fn contains_fast(cache: AnyCache, id: &str) -> bool {
        cache.contains::<Self::MidRepr>(id)
    }

    fn reload_watcher<'a>(cache: AnyCache<'a>, id: &str) -> Option<ReloadWatcher<'a>> {
        let repr = cache.get_cached::<Self::MidRepr>(id)?;
        Some(repr.reload_watcher())
    }
}

#[non_exhaustive]
pub struct GgezLoader;

pub struct ImageAsset(image::RgbaImage);

impl Asset for ImageAsset {
    type Loader = GgezLoader;
    const EXTENSIONS: &'static [&'static str] = &["png", "bmp", "wepb"];
}

impl loader::Loader<ImageAsset> for GgezLoader {
    fn load(content: Cow<[u8]>, ext: &str) -> Result<ImageAsset, BoxedError> {
        let img: image::DynamicImage = loader::ImageLoader::load(content, ext)?;
        Ok(ImageAsset(img.to_rgba8()))
    }
}

impl GgezAsset for ggez::graphics::Image {
    type MidRepr = ImageAsset;

    fn from_repr(context: &mut ggez::Context, image: &ImageAsset) -> ggez::GameResult<Self> {
        #[cold]
        fn size_error() -> ggez::GameError {
            ggez::GameError::ResourceLoadError(String::from("image dimensions do not fit a u16"))
        }

        let width = image.0.width().try_into().map_err(|_| size_error())?;
        let height = image.0.height().try_into().map_err(|_| size_error())?;

        Self::from_rgba8(context, width, height, &image.0)
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

pub struct FontAsset(Vec<u8>);

struct FontId(Mutex<ggez::graphics::Font>);

impl FontId {
    fn new(font: ggez::graphics::Font) -> Self {
        Self(parking_lot::Mutex::new(font))
    }

    fn get_font(&self) -> ggez::graphics::Font {
        *self.0.lock()
    }

    fn set_font(&self, font: ggez::graphics::Font) {
        *self.0.lock() = font;
    }
}

impl Storable for FontId {}
impl NotHotReloaded for FontId {}

impl Asset for FontAsset {
    type Loader = GgezLoader;
    const EXTENSION: &'static str = "ttf";
}

impl loader::Loader<FontAsset> for GgezLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<FontAsset, BoxedError> {
        Ok(FontAsset(content.into_owned()))
    }
}

impl GgezAsset for ggez::graphics::Font {
    type MidRepr = FontAsset;

    fn from_repr(context: &mut ggez::Context, font: &FontAsset) -> ggez::GameResult<Self> {
        Self::new_glyph_font_bytes(context, &font.0)
    }

    fn load(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let handle = cache.get_cached::<FontAsset>(id);

        // `ggez` already caches fonts so we avoid calling `new_glyph_font_bytes`
        if let Some(handle) = handle {
            if !handle.reloaded_global() {
                let font_id = cache.get_cached::<FontId>(id).ok_or_else(not_found_error)?;
                return Ok(font_id.get().get_font());
            }
        }

        let handle = match handle {
            Some(h) => h,
            None => cache.load::<FontAsset>(id).map_err(convert_error)?,
        };
        let font = Self::from_repr(context, &handle.read())?;
        let font_handle = cache.get_or_insert(id, FontId::new(font));
        font_handle.get().set_font(font);
        Ok(font)
    }

    fn load_fast(cache: AnyCache, context: &mut ggez::Context, id: &str) -> ggez::GameResult<Self> {
        let handle = cache.get_cached::<GgezValue<Self>>(id);

        if let Some(handle) = handle {
            return Ok(handle.copied().0);
        }

        let bytes = cache.load_owned::<FontAsset>(id).map_err(convert_error)?;
        let font = Self::from_owned_repr(context, bytes)?;
        cache.get_or_insert(id, GgezValue(font));
        Ok(font)
    }

    fn get_cached(
        cache: AnyCache,
        context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        let handle = cache
            .get_cached::<FontAsset>(id)
            .ok_or_else(not_found_error)?;

        if !handle.reloaded_global() {
            let font_id = cache.get_cached::<FontId>(id).ok_or_else(not_found_error)?;
            return Ok(font_id.get().get_font());
        }

        let font = Self::from_repr(context, &handle.read())?;
        let font_handle = cache.get_or_insert(id, FontId::new(font));
        font_handle.get().set_font(font);
        Ok(font)
    }

    fn get_cached_fast(
        cache: AnyCache,
        _context: &mut ggez::Context,
        id: &str,
    ) -> ggez::GameResult<Self> {
        let handle = cache
            .get_cached::<GgezValue<Self>>(id)
            .ok_or_else(not_found_error)?;
        Ok(handle.copied().0)
    }

    fn contains(cache: AnyCache, id: &str) -> bool {
        cache.contains::<FontId>(id)
    }

    fn contains_fast(cache: AnyCache, id: &str) -> bool {
        cache.contains::<GgezValue<Self>>(id)
    }
}

#[derive(Clone)]
pub struct AudioAsset(pub ggez::audio::SoundData);

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
    type MidRepr = AudioAsset;

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
    type MidRepr = AudioAsset;

    fn from_repr(context: &mut ggez::Context, sound: &AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0.clone())
    }

    fn from_owned_repr(context: &mut ggez::Context, sound: AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0)
    }
}

impl GgezAsset for ggez::audio::SpatialSource {
    type MidRepr = AudioAsset;

    fn from_repr(context: &mut ggez::Context, sound: &AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0.clone())
    }

    fn from_owned_repr(context: &mut ggez::Context, sound: AudioAsset) -> ggez::GameResult<Self> {
        Self::from_data(context, sound.0)
    }
}
