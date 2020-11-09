use rustyboi_core::emulator::{Emulator, EmulatorMode};
use rustyboi_core::InputKey;
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;

/// Represents a notification for the emulator thread to execute when possible.
#[derive(Debug)]
pub enum EmulatorNotification {
    KeyDown(InputKey),
    KeyUp(InputKey),
    /// Pass the audio buffer back and forth to avoid constant heap allocation
    AudioRequest(Vec<f32>),
    ExitRequest,
    Debug(DebugMessage),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum EmulatorResponse {
    Audio(Vec<f32>),
    Debug(DebugMessage),
}

/// Represents a special (and possibly expensive) request for debug information to the emulator
/// thread.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum DebugMessage {
    Palette(Option<PaletteDebugInfo>),
}

impl Into<EmulatorNotification> for DebugMessage {
    fn into(self) -> EmulatorNotification {
        EmulatorNotification::Debug(self)
    }
}

impl Into<EmulatorResponse> for DebugMessage {
    fn into(self) -> EmulatorResponse {
        EmulatorResponse::Debug(self)
    }
}
