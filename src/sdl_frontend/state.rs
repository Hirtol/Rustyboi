use nanoserde::{SerJson, DeJson};

#[derive(Default, Debug, Copy, Clone)]
pub struct AppEmulatorState {
    pub emulator_paused: bool,
    pub fast_forward: bool,
    pub exit: bool,
}

#[derive(Default, Debug, Copy, Clone, SerJson, DeJson)]
pub struct AppState {

}