
#[cfg(test)]
mod tests;

struct CPU {
    // Stack pointer
    sp: u16,
    // Program counter
    pc: u16,
}