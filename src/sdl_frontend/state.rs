use nanoserde::{DeJson, SerJson};

#[derive(Default, Debug, Copy, Clone)]
pub struct AppEmulatorState {
    /// Whether the emulation is currently paused.
    pub emulator_paused: bool,
    /// Whether we should fast forward at our set fast_forward_rate
    pub fast_forward: bool,
    /// Whether the emulation should run unbounded
    pub unbounded: bool,
    /// Whether the app should exit asap
    pub exit: bool,
    /// Whether we're currently awaiting audio from the emulation thread.
    pub awaiting_audio: bool,
    /// Whether we're currently awaiting debug info from the emulation thread.
    pub awaiting_debug: bool,
}

impl AppEmulatorState {
    pub fn reset(&mut self) {
        self.awaiting_debug = false;
        self.awaiting_audio = false;
        self.emulator_paused = false;
    }
}

#[derive(Debug, Copy, Clone, SerJson, DeJson)]
pub struct AppState {
    /// The speed multiplier to use while fast forwarding.
    pub fast_forward_rate: u64,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            fast_forward_rate: 2,
            .. Default::default()
        }
    }
}