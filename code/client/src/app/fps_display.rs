use macroquad::{text::TextParams, time::get_fps};

use crate::ui::{
    CANONICAL_SCREEN_WIDTH, TEXT_SMALL, Text, TextHorizontalPositioning, TextVerticalPositioning,
    default_text_params,
};

pub(crate) struct FPSDisplay {
    last_measures: Vec<i32>,
    current: usize,
    smoothing: usize,
}

impl FPSDisplay {
    pub fn new(smoothing: usize) -> Self {
        Self {
            last_measures: vec![0; smoothing],
            current: 0,
            smoothing,
        }
    }

    pub fn update(&mut self) {
        self.last_measures[self.current] = get_fps();
        self.current = (self.current + 1) % self.smoothing;
    }

    pub fn draw(&self) {
        let current = self.last_measures.iter().sum::<i32>() / self.smoothing as i32;
        let text = Text::new(
            TextParams {
                font_size: TEXT_SMALL,
                ..default_text_params()
            },
            TextVerticalPositioning::CenterConsistent,
            TextHorizontalPositioning::Right,
        );

        // FPS is always drawn in the top-right corner
        text.draw_scaled_no_offset(&current.to_string(), CANONICAL_SCREEN_WIDTH - 30., 30.);
    }
}
