use ggez::{
    audio::{self, SoundSource},
    event,
    graphics::{self, Drawable, Image},
    GameResult,
};
use ggez_assets_manager::AssetCacheExt;

const GAME_ID: &str = "ggez";
const AUTHOR: &str = "assets_manager";

const MAIN_TEXT: &str = if cfg!(feature = "hot-reloading") {
    "Press SPACE to play a sound\n\nTry changing resource files\nto test hot-reloading"
} else {
    "Press SPACE to play a sound\n\nHot-reloading is disabled"
};

struct MainState {
    cache: ggez_assets_manager::GgezAssetCache,
}

impl MainState {
    fn new() -> Self {
        Self {
            cache: ggez_assets_manager::new_asset_cache(GAME_ID, AUTHOR),
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
        let font = self.cache.ggez_load(ctx, "fonts.DejaVuSans")?;
        let img = self.cache.ggez_load::<Image>(ctx, "images.ferris")?;

        let graphics::Rect { w, .. } = graphics::screen_coordinates(ctx);

        graphics::clear(ctx, graphics::Color::BLACK);

        // In a real game this would probably be cached
        graphics::Text::new(MAIN_TEXT)
            .set_font(font, graphics::PxScale::from(20.))
            .set_bounds([w, f32::INFINITY], graphics::Align::Center)
            .draw(ctx, graphics::DrawParam::from(([0., 50.],)))?;

        graphics::draw(ctx, &img, ([(w - img.width() as f32) / 2., 200.],))?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        keycode: event::KeyCode,
        _keymods: event::KeyMods,
        repeat: bool,
    ) {
        if keycode == event::KeyCode::Space && !repeat {
            match self.cache.ggez_load::<audio::Source>(ctx, "audio.on_key") {
                Ok(mut source) => source.play_detached(ctx).unwrap(),
                Err(err) => {
                    static LOGGED: parking_lot::Once = parking_lot::Once::new();
                    LOGGED.call_once(|| log::error!("Failed to load sound: {}", err));
                }
            }
        }
    }
}

fn main() -> GameResult<()> {
    env_logger::init();
    let (ctx, event_loop) = ggez::ContextBuilder::new(GAME_ID, AUTHOR).build()?;
    let state = MainState::new();

    event::run(ctx, event_loop, state)
}
