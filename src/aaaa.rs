use assets_manager::{
    asset::DirLoadable, loader, AnyCache, Asset, AssetReadGuard, Compound, Handle, OnceInitCell,
    ReloadWatcher,
};
use ggez::{GameError, GameResult};

pub trait GgezAsset: Sized + Send + Sync + 'static {
    type Raw: Compound;

    fn from_raw(raw: Self::Raw, ctx: &mut ggez::Context) -> GameResult<Self>;
}

pub struct Storage<T: GgezAsset>(OnceInitCell<Option<T::Raw>, T>);

impl<T: GgezAsset> Compound for Storage<T> {
    fn load(
        cache: assets_manager::AnyCache,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        Compound::load(cache, id).map(Self)
    }
}

impl<T: GgezAsset> DirLoadable for Storage<T>
where
    T::Raw: DirLoadable,
{
    fn select_ids(
        cache: assets_manager::AnyCache,
        id: &assets_manager::SharedString,
    ) -> std::io::Result<Vec<assets_manager::SharedString>> {
        T::Raw::select_ids(cache, id)
    }

    fn sub_directories(
        cache: assets_manager::AnyCache,
        id: &assets_manager::SharedString,
        f: impl FnMut(&str),
    ) -> std::io::Result<()> {
        T::Raw::sub_directories(cache, id, f)
    }
}

impl<T: GgezAsset> Storage<T> {
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }

    pub fn get_or_init(&self, ctx: &mut ggez::Context) -> GameResult<&T> {
        self.0.get_or_try_init(|raw| {
            let raw = raw
                .take()
                .ok_or_else(|| GameError::ResourceLoadError(String::from("nope")))?;
            T::from_raw(raw, ctx)
        })
    }
}

pub trait HandleExt {
    type Target;

    fn read_get(&self) -> Option<AssetReadGuard<'_, Self::Target>>;

    fn read_or_init(&self, ctx: &mut ggez::Context)
        -> GameResult<AssetReadGuard<'_, Self::Target>>;
}

impl<T: GgezAsset> HandleExt for Handle<Storage<T>> {
    type Target = T;

    fn read_get(&self) -> Option<AssetReadGuard<'_, T>> {
        AssetReadGuard::try_map(self.read(), |x| x.get()).ok()
    }

    fn read_or_init(&self, ctx: &mut ggez::Context) -> GameResult<AssetReadGuard<'_, T>> {
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

trait AssetCacheExt {
    fn as_any_cache(&self) -> AnyCache<'_>;

    fn ggez_load_clone<T>(&self, id: &str, ctx: &mut ggez::Context) -> GameResult<T>
    where
        T: GgezAsset + Clone,
    {
        let cache = self.as_any_cache();
        let h = cache
            .load::<Storage<T>>(id)
            .map_err(crate::assets::convert_error)?;
        let asset = h.read().get_or_init(ctx)?.clone();
        Ok(asset)
    }

    fn ggez_load<T>(&self, id: &str, ctx: &mut ggez::Context) -> GameResult<T>
    where
        T: GgezAsset + Clone,
    {
        let cache = self.as_any_cache();
        let h = cache
            .load::<Storage<T>>(id)
            .map_err(crate::assets::convert_error)?;
        let asset = h.read().get_or_init(ctx)?.clone();
        Ok(asset)
    }

    fn ggez_reload_watcher<T>(&self, id: &str) -> Option<ReloadWatcher<'_>>
    where
        T: GgezAsset,
    {
        let cache = self.as_any_cache();
        let h = cache.get_cached::<Storage<T>>(id)?;
        Some(h.reload_watcher())
    }
}

pub struct ImageAsset(image::RgbaImage);

impl From<image::DynamicImage> for ImageAsset {
    fn from(img: image::DynamicImage) -> Self {
        ImageAsset(img.into_rgba8())
    }
}

impl Asset for ImageAsset {
    type Loader = loader::LoadFrom<image::DynamicImage, loader::ImageLoader>;
    const EXTENSIONS: &'static [&'static str] = &["png", "bmp", "wepb", "jpeg", "jpg"];
}

impl GgezAsset for ggez::graphics::Image {
    type Raw = ImageAsset;

    fn from_raw(image: ImageAsset, ctx: &mut ggez::Context) -> GameResult<Self> {
        Ok(ggez::graphics::Image::from_pixels(
            ctx,
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

impl GgezAsset for ggez::graphics::Shader {
    type Raw = ShaderAsset;

    fn from_raw(shader: ShaderAsset, ctx: &mut ggez::Context) -> GameResult<Self> {
        ggez::graphics::ShaderBuilder::new()
            .fragment_code(&shader.0)
            .vertex_code(&shader.0)
            .build(ctx)
    }
}
