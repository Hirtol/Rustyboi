use crate::state::DisplayColourConfigurable;
use rustyboi_core::emulator::GameBoyModel;
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;
use rustyboi_core::InputKey;

/// Represents a notification for the emulator thread to execute when possible.
#[derive(Debug)]
pub enum EmulatorNotification {
    KeyDown(InputKey),
    KeyUp(InputKey),
    /// Pass the audio buffer back and forth to avoid constant heap allocation
    AudioRequest(Vec<f32>),
    ExtraAudioRequest,
    ExitRequest,
    Debug(DebugMessage),
    ChangeDisplayColour(DisplayColourConfigurable),
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
    Mode(Option<GameBoyModel>),
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
