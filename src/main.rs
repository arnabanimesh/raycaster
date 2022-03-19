use ggez::conf::{WindowMode, WindowSetup};
use ggez::{event, ContextBuilder, GameResult};
use raycaster::MainState;

const TITLE: &str = "RayCaster";

fn main() -> GameResult {
    let window_mode = WindowMode::default().dimensions(1200.0, 800.0);
    let window_setup = WindowSetup::default().title(TITLE);
    let (mut ctx, events_loop) = ContextBuilder::new(TITLE, "migue")
        .window_mode(window_mode)
        .window_setup(window_setup)
        .add_resource_path("assets")
        .build()?;
    let main_state = MainState::new(&mut ctx)?;
    event::run(ctx, events_loop, main_state)
}
