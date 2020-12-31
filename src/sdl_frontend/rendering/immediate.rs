use sdl2::mouse::MouseState;

use crate::communication::DebugMessage;
use rustyboi::storage::FileStorage;
use sdl2::event::Event;
use std::sync::Arc;

pub enum UiEvent {
    EmulatorUpdate,
    GeneralUpdate,
}

pub trait ImmediateGui {
    /// Create a new instance of the type implementing `ImmediateGui`.
    fn new(
        video_subsystem: &sdl2::VideoSubsystem,
        host_window: &sdl2::video::Window,
        storage: Arc<FileStorage>,
    ) -> Self;

    /// Returns a list of `DebugRequests` the GUI wishes to have fulfilled.
    /// Requests can be dropped, in which case the GUI shouldn't display anything and will send the
    /// request again at the next opportunity.
    fn query_emulator(&mut self) -> Vec<DebugMessage>;

    /// Fulfills the GUI's request presented at `query_emulator`.
    fn fulfill_query(&mut self, debug_response: DebugMessage);

    fn prepare_render(&mut self, delta_time: f32, host_window: &sdl2::video::Window, mouse_state: &MouseState);

    fn render(&mut self, host_window: &sdl2::video::Window);

    /// Handles `SDL` events for things like keypresses/mouse movement
    fn handle_event(&mut self, event: &Event);
}
