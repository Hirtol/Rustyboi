use rustyboi_core::emulator::Emulator;
use rustyboi_core::InputKey;

/// Represents a notification for the emulator thread to execute when possible.
pub enum EmulatorNotification {
    KeyDown(InputKey),
    KeyUp(InputKey),
    /// Pass the audio buffer back and forth to avoid constant heap allocation
    AudioRequest(Vec<f32>),
    ExitRequest,
    DebugRequest(DebugRequest),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum EmulatorResponse {
    Audio(Vec<f32>),
    DebugResponse(DebugResponse),
}

/// Represents a special (and possibly expensive) request for debug information to the emulator
/// thread.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum DebugRequest {
    Palette,
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum DebugResponse {
    Palette
}
