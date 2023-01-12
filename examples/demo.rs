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
    cache: ggez_assets_manager::GgezAssetCache,
}

impl MainState {
    fn new(ctx: &ggez::Context) -> Self {
        Self {
            cache: ggez_assets_manager::create_asset_cache(ctx),
        }
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> GameResult<()> {
        #[cfg(feature = "hot-reloading")]
        self.cache.hot_reload();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult<()> {
        self.cache.set_font(ctx, FONT_NAME, "fonts.DejaVuSans")?;
        let img = self
            .cache
            .ggez_load::<graphics::Image>(ctx, "images.ferris")?;

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
        if input.keycode == Some(ggez::input::keyboard::KeyCode::Space) && !repeated {
            match self.cache.ggez_load::<audio::Source>(ctx, "audio.on_key") {
                Ok(mut source) => source.play_detached(ctx)?,
                Err(err) => {
                    static LOGGED: parking_lot::Once = parking_lot::Once::new();
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
        .init();

    let (ctx, event_loop) = ggez::ContextBuilder::new(GAME_ID, AUTHOR).build()?;
    let state = MainState::new(&ctx);

    event::run(ctx, event_loop, state)
}
