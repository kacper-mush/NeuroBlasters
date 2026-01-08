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

    /// Record a specific FPS value (useful for testing)
    #[cfg(test)]
    pub fn record(&mut self, fps: i32) {
        self.last_measures[self.current] = fps;
        self.current = (self.current + 1) % self.smoothing;
    }

    /// Get the smoothed FPS value
    pub fn smoothed_fps(&self) -> i32 {
        self.last_measures.iter().sum::<i32>() / self.smoothing as i32
    }

    pub fn draw(&self) {
        let current = self.smoothed_fps();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fps_display_initial_zero() {
        let display = FPSDisplay::new(5);
        assert_eq!(display.smoothed_fps(), 0);
    }

    #[test]
    fn test_fps_display_single_value() {
        let mut display = FPSDisplay::new(5);
        display.record(100);
        // 100 / 5 = 20 (only one sample, rest are zeros)
        assert_eq!(display.smoothed_fps(), 20);
    }

    #[test]
    fn test_fps_display_full_buffer() {
        let mut display = FPSDisplay::new(5);
        for _ in 0..5 {
            display.record(60);
        }
        assert_eq!(display.smoothed_fps(), 60);
    }

    #[test]
    fn test_fps_display_averaging() {
        let mut display = FPSDisplay::new(4);
        display.record(40);
        display.record(60);
        display.record(80);
        display.record(100);
        // (40 + 60 + 80 + 100) / 4 = 70
        assert_eq!(display.smoothed_fps(), 70);
    }

    #[test]
    fn test_fps_display_circular_buffer() {
        let mut display = FPSDisplay::new(3);
        display.record(30);
        display.record(30);
        display.record(30);
        assert_eq!(display.smoothed_fps(), 30);

        // Now record more values to overwrite old ones
        display.record(60);
        display.record(60);
        display.record(60);
        assert_eq!(display.smoothed_fps(), 60);
    }

    #[test]
    fn test_fps_display_smoothing_size_one() {
        let mut display = FPSDisplay::new(1);
        display.record(120);
        assert_eq!(display.smoothed_fps(), 120);

        display.record(30);
        assert_eq!(display.smoothed_fps(), 30);
    }
}
