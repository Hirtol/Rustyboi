pub trait AudioVoice {

}

pub struct Voice1 {
    /// 0xFF10 -PPP NSSS  Sweep period, negate, shift
    nr10: u8,
    /// 0xFF11 DDLL LLLL  Duty, Length load (64-L)
    nr11: u8,
    /// 0xFF12 VVVV APPP  Starting volume, Envelope add mode, period
    nr12: u8,
    /// 0xFF13 FFFF FFFF  Frequency LSB
    nr13: u8,
    /// 0xFF14 TL-- -FFF  Trigger, Length enable, Frequency MSB
    nr14: u8,
}

/// Relevant for voice 1 and 2 for the DMG.
///
/// # Properties:
/// * Sweep (only voice 1)
/// * Volume Envelope
/// * Length Counter
pub struct SquareWaveChannel {
    has_sweep: bool,
}

/// Relevant for voice 3 for the DMG.
///
/// # Properties:
/// * Length Counter
pub struct WaveformChannel {

}
/// Relevant for voice 4 for the DMG.
///
/// # Properties:
/// * Volume Envelope
pub struct NoiseChannel {

}