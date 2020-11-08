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
}

#[derive(Default, Debug, Copy, Clone, SerJson, DeJson)]
pub struct AppState {
    /// The speed multiplier to use while fast forwarding.
    pub fast_forward_rate: u64,
}
