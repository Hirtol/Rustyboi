use nanoserde::{DeJson, SerJson};

#[derive(Default, Debug, Copy, Clone, DeJson, SerJson)]
pub struct State {
    example: u32,
    hello: bool,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default()
    }
}