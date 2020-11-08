use sdl2::mouse::MouseState;

use rustyboi::storage::FileStorage;
use std::sync::Arc;
use sdl2::event::Event;

pub trait ImmediateGui {
    fn new(
        video_subsystem: &sdl2::VideoSubsystem,
        host_window: &sdl2::video::Window,
        storage: Arc<FileStorage>,
    ) -> Self;
    fn query_emulator(&mut self);
    fn prepare_render(&mut self, delta_time: f32, host_window: &sdl2::video::Window, mouse_state: &MouseState);
    fn render(&mut self, host_window: &sdl2::video::Window);
    fn handle_event(&mut self, event: &Event);
}
