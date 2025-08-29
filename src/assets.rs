use crate::GgezAsset;
use assets_manager::{AssetCache, BoxedError, FileAsset};
use std::{borrow::Cow, io, sync::Mutex};

/// Converts an error from `assets_manager` to `ggez`
#[cold]
pub fn convert_error(err: assets_manager::Error) -> ggez::GameError {
    match err.reason().downcast_ref::<io::Error>() {
        Some(io_err) if io_err.kind() == io::ErrorKind::NotFound => {
            ggez::GameError::ResourceNotFound(err.id().to_string(), Vec::new())
        }
        _ => ggez::GameError::ResourceLoadError(format!("\"{}\": {:?}", err.id(), err.reason())),
    }
}

pub struct ImageAsset(Vec<u8>);

impl FileAsset for ImageAsset {
    const EXTENSIONS: &'static [&'static str] = &["png", "bmp", "wepb", "jpeg", "jpg"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(ImageAsset(bytes.into_owned()))
    }
}

impl GgezAsset for ggez::graphics::Image {
    type Raw = ImageAsset;

    fn from_raw(image: &mut ImageAsset, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        ggez::graphics::Image::from_bytes(ctx, &image.0)
    }
}

pub struct ShaderAsset(String);

impl FileAsset for ShaderAsset {
    const EXTENSION: &'static str = "wgsl";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        String::from_bytes(bytes).map(ShaderAsset)
    }
}

impl GgezAsset for ggez::graphics::Shader {
    type Raw = ShaderAsset;

    fn from_raw(shader: &mut ShaderAsset, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        ggez::graphics::ShaderBuilder::new()
            .fragment_code(&shader.0)
            .vertex_code(&shader.0)
            .build(ctx)
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
pub struct AudioAsset(pub ggez::audio::SoundData);

impl FileAsset for AudioAsset {
    const EXTENSIONS: &'static [&'static str] = &["ogg", "flac", "wav"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(AudioAsset(ggez::audio::SoundData::from_bytes(&bytes)?))
    }
}

impl GgezAsset for ggez::audio::SoundData {
    type Raw = AudioAsset;

    fn from_raw(raw: &mut Self::Raw, _: &mut ggez::Context) -> ggez::GameResult<Self> {
        Ok(raw.0.clone())
    }
}

impl GgezAsset for ggez::audio::Source {
    type Raw = AudioAsset;

    fn from_raw(raw: &mut Self::Raw, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        Self::from_data(ctx, raw.0.clone())
    }
}

impl GgezAsset for ggez::audio::SpatialSource {
    type Raw = AudioAsset;

    fn from_raw(raw: &mut Self::Raw, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        Self::from_data(ctx, raw.0.clone())
    }
}
