use rustyboi_core::InputKey;
use rustyboi_core::emulator::Emulator;

/// Represents a notification for the emulator thread to execute when possible.
pub enum EmulatorNotification {
    KeyDown(InputKey),
    KeyUp(InputKey),
    /// Pass the audio buffer back and forth to avoid constant heap allocation
    AudioRequest(Vec<f32>),
    ExitRequest(Box<dyn Fn(&Emulator) + Send>),
    Request(DebugRequest)
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum EmulatorResponse {
    AUDIO(Vec<f32>),
}

/// Represents a special (and possibly expensive) request for debug information to the emulator
/// thread.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum DebugRequest {
    PALETTE
}