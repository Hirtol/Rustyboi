use nanoserde::{DeJson, SerJson};

#[derive(Default, Debug, Copy, Clone, DeJson, SerJson)]
pub struct State {
    pub show_metrics: bool,
    pub palette_window: bool,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default()
    }
}
