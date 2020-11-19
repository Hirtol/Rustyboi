use crate::rendering::imgui::animate::formulas::TimeStepFormula;
use imgui::Ui;
use std::fmt::Debug;
use std::time::Duration;

#[derive(Copy, Clone, Debug, Default)]
pub struct FadeAnimation<T: TimeStepFormula> {
    animation_progress: f64,
    start_time: f64,
    remaining_duration: f64,
    duration: f64,
    formula: T,
}

impl<T: TimeStepFormula> FadeAnimation<T> {
    pub fn new(ui: &Ui, duration: Duration) -> Self {
        FadeAnimation {
            animation_progress: 1.0,
            start_time: ui.time(),
            duration: duration.as_secs_f64(),
            remaining_duration: duration.as_secs_f64(),
            formula: T::default(),
        }
    }

    /// Returns the current progress of the animation, to be used as Alpha style value.
    /// Look at `progress_animation()` to actually... progress the animation.
    pub fn progress(&self) -> f32 {
        self.animation_progress as f32
    }

    pub fn finished(&self) -> bool {
        self.animation_progress == 0.0
    }

    /// Advance the animation, depends on the `ImGui` time, so can be called as often
    /// as need be.
    pub fn progress_animation(&mut self, ui: &Ui) {
        let time_slice = (ui.time() - self.start_time) / self.remaining_duration;
        self.animation_progress = 1.0 - T::step(time_slice);
    }

    /// Reset the fade, but cut the animation duration in half.
    pub fn partial_reset(&mut self, ui: &Ui) {
        self.reset(ui);
        self.remaining_duration = self.duration / 2.0;
    }

    /// Reset the fade to the beginning.
    pub fn reset(&mut self, ui: &Ui) {
        self.start_time = ui.time();
        self.animation_progress = 1.0
    }
}

pub mod formulas {
    use std::fmt::Debug;

    pub trait TimeStepFormula: Copy + Clone + Default + Debug {
        fn step(time_slice: f64) -> f64;
    }

    #[derive(Copy, Clone, Default, Debug)]
    pub struct ParametricBlend;
    #[derive(Copy, Clone, Default, Debug)]
    pub struct Quadratic;
    #[derive(Copy, Clone, Default, Debug)]
    pub struct Linear;

    impl TimeStepFormula for ParametricBlend {
        fn step(time_slice: f64) -> f64 {
            let square = time_slice * time_slice;
            square / (2.0 * (square - time_slice) + 1.0)
        }
    }

    impl TimeStepFormula for Quadratic {
        fn step(time_slice: f64) -> f64 {
            time_slice * time_slice
        }
    }

    impl TimeStepFormula for Linear {
        fn step(time_slice: f64) -> f64 {
            time_slice
        }
    }
}
