use crate::rendering::immediate::ImmediateGui;
use sdl2::video::Window;
use sdl2::mouse::MouseState;
use sdl2::VideoSubsystem;
use egui::color;
use std::sync::Arc;

pub struct EguiBoi {
    painter: egui_sdl2_gl::Painter,
    egui_context: Arc<egui::Context>,
    pub raw_input: egui::RawInput,
    pixels_per_point: f32,
}

impl ImmediateGui for EguiBoi {
    fn new(video_subsystem: &VideoSubsystem, host_window: &Window) -> Self {
        let mut painter = egui_sdl2_gl::Painter::new(&video_subsystem, 800, 720);

        let pixels_per_point = 96f32 / video_subsystem.display_dpi(0).unwrap().0;

        let mut raw_input = egui::RawInput {
            screen_size: {
                let (width, height) = host_window.size();
                egui::vec2(width as f32, height as f32) / pixels_per_point
            },
            pixels_per_point: Some(pixels_per_point),
            ..Default::default()
        };
        
        EguiBoi {
            painter,
            egui_context: egui::Context::new(),
            raw_input,
            pixels_per_point,
        }
    }

    fn query_emulator(&mut self) {
        unimplemented!()
    }

    fn prepare_render(&mut self, delta_time: f32, host_window: &Window, mouse_state: &MouseState) {
        self.raw_input.time = delta_time as f64;
    }

    fn render(&mut self, host_window: &Window) {
        let ui = self.egui_context.begin_frame(self.raw_input.take());
        egui::Window::new("Egui with SDL2 events and GL texture").show(ui.ctx(), |ui| {
            //Image just needs a texture id reference, so we just pass it the texture id that was returned to us
            //when we previously initialized the texture.
            //ui.add(Image::new(chip8_tex_id, egui::vec2(500 as f32, 256 as f32)));
            ui.separator();
            ui.label("A simple sine wave plotted via some probably dodgy math. The GL texture is dynamically updated and blitted to an Egui managed Image.");
            ui.label(" ");
            ui.add(egui::Slider::f32(&mut 0.0, 0.0..=100.0).text("Amplitude"));
            ui.label(" ");
            if ui.button("Quit").clicked {
                std::process::exit(0);
            }
        });

        //We aren't handling the output at the moment.
        let (_output, paint_jobs) = self.egui_context.end_frame();

        self.painter.paint_jobs(color::srgba(0, 0, 255, 128), paint_jobs, &self.egui_context.texture(), self.pixels_per_point);
    }
}

impl Drop for EguiBoi {
    fn drop(&mut self) {
        self.painter.cleanup();
    }
}