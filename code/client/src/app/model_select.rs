use crate::app::training_mode_select::TrainingModeSelect;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{self};
use crate::ui::{BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_HEIGHT, CANONICAL_SCREEN_WIDTH};
use burn::backend::Wgpu;
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use common::rl::BotBrain;
use macroquad::miniquad::gl::{GL_SCISSOR_TEST, glDisable, glEnable, glScissor};
use macroquad::prelude::*;
use std::fs;

type ClientBackend = Wgpu;

pub(crate) struct ModelSelect {
    files: Vec<String>,
    scroll: f32,
    back_clicked: bool,
    picked_file: Option<String>,
}

impl ModelSelect {
    pub fn new() -> Self {
        Self::refresh_file_list()
    }

    fn refresh_file_list() -> Self {
        let mut files = Vec::new();
        let path = "assets/models";

        let _ = fs::create_dir_all(path);

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type()
                    && ft.is_file()
                    && let Some(fname) = entry.file_name().to_str()
                    && fname.ends_with(".bin")
                {
                    files.push(fname.to_string());
                }
            }
        }
        files.sort();

        Self {
            files,
            scroll: 0.0,
            back_clicked: false,
            picked_file: None,
        }
    }
}

impl View for ModelSelect {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        let (_, y_scroll) = mouse_wheel();
        if y_scroll != 0.0 {
            self.scroll -= y_scroll * 30.0;

            let layout_padding = 15.0;
            let item_height = BUTTON_H + layout_padding;
            let content_height = self.files.len() as f32 * item_height;
            let view_height = CANONICAL_SCREEN_HEIGHT - 250.0;
            let max_scroll = (content_height - view_height + 50.0).max(0.0); // +50 padding for last item visibility

            self.scroll = self.scroll.clamp(0.0, max_scroll);
        }

        // Handle Back Button Logic
        if self.back_clicked {
            self.back_clicked = false;
            return Transition::Pop;
        }

        // Handle file pick logic
        if let Some(fname) = self.picked_file.take() {
            let full_path = format!("assets/models/{}", fname);
            let load_name = if fname.ends_with(".bin") {
                &full_path[..full_path.len() - 4]
            } else {
                &full_path
            };
            let recorder = BinFileRecorder::<FullPrecisionSettings>::default();
            if let Ok(brain) = BotBrain::<ClientBackend>::new(&Default::default()).load_file(
                load_name,
                &recorder,
                &Default::default(),
            ) {
                return Transition::Push(Box::new(TrainingModeSelect::new(fname, brain)));
            }
        }

        Transition::None
    }

    fn draw(&mut self, _ctx: &AppContext, has_input: bool) {
        let x_mid = CANONICAL_SCREEN_WIDTH / 2.;
        let mut layout = ui::Layout::new(80., 15.);

        ui::Text::new_title().draw("Select Model", x_mid, layout.next());
        layout.add(60.);

        ui::Text::new_scaled(ui::TEXT_MID).draw("Existing Models:", x_mid, layout.next());
        layout.add(30.);

        // --- SCROLLABLE AREA START ---
        let list_start_y = layout.next();
        let back_button_y = CANONICAL_SCREEN_HEIGHT - 80.0; // Fixed position for Back button
        let list_end_y = back_button_y - 20.0;

        // Define clipping region (Scissor)
        let (scale, x_off, y_off) =
            ui::calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT);
        let sc_y_start = list_start_y * scale + y_off;

        // Calculate dynamic height for the view area
        let view_height_canonical = list_end_y - list_start_y;
        let sc_h = view_height_canonical * scale;

        let sc_x = 0.0 * scale + x_off; // Full width
        let sc_w = CANONICAL_SCREEN_WIDTH * scale;

        let screen_h_px = screen_height();
        let gl_y = screen_h_px - (sc_y_start + sc_h);

        unsafe {
            get_internal_gl().flush();
            glScissor(sc_x as i32, gl_y as i32, sc_w as i32, sc_h as i32);
            glEnable(GL_SCISSOR_TEST);
        }

        // Draw List
        let mut list_layout = ui::Layout::new(list_start_y - self.scroll, 15.);
        self.picked_file = None;

        for file in self.files.iter() {
            // Culling: Only draw if roughly in view
            let item_y = list_layout.next();
            // Draw if the item is at least partially visible
            if item_y + BUTTON_H > list_start_y
                && item_y < list_end_y
                && Button::default()
                    .draw_centered(
                        x_mid,
                        item_y,
                        BUTTON_W * 1.5,
                        BUTTON_H,
                        Some(file),
                        has_input,
                    )
                    .poll()
            {
                self.picked_file = Some(file.clone());
            }
            list_layout.add(BUTTON_H);
        }

        unsafe {
            get_internal_gl().flush();
            glDisable(GL_SCISSOR_TEST);
        }
        // --- SCROLLABLE AREA END ---

        // Back Button (Fixed)
        self.back_clicked = Button::default()
            .draw_centered(
                x_mid,
                back_button_y,
                BUTTON_W,
                BUTTON_H,
                Some("Back"),
                has_input,
            )
            .poll();
    }

    fn get_id(&self) -> ViewId {
        ViewId::ModelSelect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_refresh_file_list() {
        let test_filename = "test_model_12345.bin";
        let path = format!("assets/models/{}", test_filename);

        let _ = fs::create_dir_all("assets/models");
        {
            let mut file = File::create(&path).expect("Failed to create test file");
            file.write_all(b"dummy data").unwrap();
        }

        let menu = ModelSelect::refresh_file_list();

        assert!(
            menu.files.contains(&test_filename.to_string()),
            "File list should contain created test file"
        );

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
