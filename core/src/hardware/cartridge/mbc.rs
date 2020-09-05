enum MBC {
    NONE,
    MBC1 {ram_enabled: bool, },
    MBC2,
    MBC3,
    MBC5,

    MBC6,
    MBC7,
    HuC1,
}

/// Struct representing No MBC
pub struct MBC0 {
    rom: Vec<u8>
}

pub struct MBC1<'a> {
    ram_enabled: bool,
    rom: &'a[u8],

}