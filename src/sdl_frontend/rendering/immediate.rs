use crossbeam::channel::*;
use sdl2::mouse::MouseState;
use sdl2::video::GLContext;
use std::sync::Arc;
use rustyboi::storage::{Storage, FileStorage};

pub trait ImmediateGui {
    fn new(video_subsystem: &sdl2::VideoSubsystem, host_window: &sdl2::video::Window, storage: Arc<FileStorage>) -> Self;
    fn query_emulator(&mut self);
    fn prepare_render(&mut self, delta_time: f32, host_window: &sdl2::video::Window, mouse_state: &MouseState);
    fn render(&mut self, host_window: &sdl2::video::Window);
}