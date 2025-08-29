use ggez::{
    audio::{self, SoundSource},
    event, graphics, GameResult,
};
use ggez_assets_manager::AssetCacheExt;

const GAME_ID: &str = "ggez";
const AUTHOR: &str = "assets_manager";

const MAIN_TEXT: &str = if cfg!(feature = "hot-reloading") {
    "Press SPACE to play a sound\n\nTry changing resource files\nto test hot-reloading"
} else {
    "Press SPACE to play a sound\n\nHot-reloading is disabled"
};

const FONT_NAME: &str = "default_font";

struct MainState {
    cache: ggez_assets_manager::AssetCache,
}

impl MainState {
    fn new(ctx: &ggez::Context) -> Self {
        Self {
            cache: ggez_assets_manager::new_asset_cache(ctx),
        }
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> GameResult<()> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        self.cache.set_font(ctx, FONT_NAME, "fonts.DejaVuSans")?;
        let img = self
            .cache
            .ggez_load_clone::<graphics::Image>(ctx, "images.ferris")?;

        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);

        let (w, _h) = ctx.gfx.drawable_size();

        // In a real game this would probably be cached
        let mut text = graphics::Text::new(MAIN_TEXT);
        text.set_font(FONT_NAME)
            .set_layout(graphics::TextLayout {
                h_align: graphics::TextAlign::Middle,
                v_align: graphics::TextAlign::Begin,
            })
            .set_scale(25.);
        canvas.draw(&text, [w / 2., 50.]);

        canvas.draw(&img, [(w - img.width() as f32) / 2., 200.]);

        canvas.finish(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        input: ggez::input::keyboard::KeyInput,
        repeated: bool,
    ) -> GameResult<()> {
        if input.event.physical_key == ggez::input::keyboard::KeyCode::Space && !repeated {
            match self
                .cache
                .ggez_load_init::<audio::Source>(ctx, "audio.on_key")
            {
                Ok(source) => source.play(),
                Err(err) => {
                    static LOGGED: std::sync::Once = std::sync::Once::new();
                    LOGGED.call_once(|| log::error!("Failed to load sound: {}", err));
                }
            }
        }

        Ok(())
    }
}

fn main() -> GameResult<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("symphonia_core", log::LevelFilter::Warn)
        .filter_module("symphonia_format_ogg", log::LevelFilter::Warn)
        .init();

    let mut ctx_builder = ggez::ContextBuilder::new(GAME_ID, AUTHOR);

    // By default, `ggez` searches resources directory next to the executable,
    // so override this.
    if let Some(mut path) = std::env::var_os("CARGO_MANIFEST_DIR") {
        path.push("/resources");
        ctx_builder = ctx_builder.resources_dir_name(path);
    }

    let (ctx, event_loop) = ctx_builder.build()?;
    let state = MainState::new(&ctx);

    event::run(ctx, event_loop, state)
}
