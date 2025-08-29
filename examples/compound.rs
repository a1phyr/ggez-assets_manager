use assets_manager::{Asset, AssetCache, SharedString};
use ggez_assets_manager::{ArcGgezHandle, AssetCacheExt, GgezAsset};

/// The `Person` as it is stored
#[derive(serde::Deserialize, assets_manager::Asset)]
#[asset_format = "ron"]
struct RonPerson {
    name: String,
    avatar: SharedString,
}

/// The `Person` as it is used
#[allow(dead_code)]
struct Person {
    name: String,
    avatar: ArcGgezHandle<ggez::graphics::Image>,
}

impl Asset for Person {
    fn load(
        cache: &AssetCache,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        let RonPerson { name, avatar } = RonPerson::load(cache, id)?;
        Ok(Person {
            name,
            avatar: cache.load(&avatar)?.strong(),
        })
    }
}

// Bonus: how to implement `GgezAsset` if you need it

/// The "raw" value, that doesn't require a context
struct PersonRaw {
    name: String,
    avatar: <ggez::graphics::Image as GgezAsset>::Raw,
}

impl Asset for PersonRaw {
    fn load(
        cache: &AssetCache,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        let RonPerson { name, avatar } = RonPerson::load(cache, id)?;
        Ok(PersonRaw {
            name,
            avatar: Asset::load(cache, &avatar)?,
        })
    }
}

/// The actual type that implements `GgezAsset`.
#[allow(dead_code)]
struct GgezPerson {
    name: String,
    avatar: ggez::graphics::Image,
}

impl GgezAsset for GgezPerson {
    type Raw = PersonRaw;

    fn from_raw(raw: &mut PersonRaw, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        Ok(GgezPerson {
            name: std::mem::take(&mut raw.name),
            avatar: GgezAsset::from_raw(&mut raw.avatar, ctx)?,
        })
    }
}

fn main() -> ggez::GameResult<()> {
    let mut ctx_builder = ggez::ContextBuilder::new("compounds-demo", "assets_manager");

    // By default, `ggez` searches resources directory next to the executable,
    // so override this.
    if let Some(mut path) = std::env::var_os("CARGO_MANIFEST_DIR") {
        path.push("/resources");
        ctx_builder = ctx_builder.resources_dir_name(path);
    }

    let (ctx, _) = ctx_builder.build()?;

    let cache = AssetCache::new_ggez(&ctx);

    // Demo: how to load the first type
    let _ = cache
        .load::<Person>("")
        .map_err(ggez_assets_manager::convert_error)?;

    // Demo: how to load the second type
    let _ = cache.ggez_load::<GgezPerson>("")?;

    Ok(())
}
