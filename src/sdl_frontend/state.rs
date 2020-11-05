
#[derive(Default, Debug, Copy, Clone)]
pub struct AppState {
    pub emulator_paused: bool,
    pub fast_forward: bool,
    pub exit: bool,
}