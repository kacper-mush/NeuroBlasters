use crate::ui::{
    CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, TEXT_SMALL, Text, TextHorizontalPositioning,
    TextVerticalPositioning, default_text_params,
};
use macroquad::prelude::*;

pub(crate) struct MainFeed {
    text: String,
}

impl MainFeed {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn set(&mut self, text: String) {
        self.text = text;
    }

    pub fn draw(&self) {
        Text::new_scaled(TEXT_LARGE).draw(&self.text, CANONICAL_SCREEN_MID_X, 50.);
    }
}

struct FeedElement {
    text: String,
    active_time: f64,
    time_start: Option<f64>,
    is_active: bool,
}

impl FeedElement {
    fn new(text: String, expire_time: f64) -> Self {
        Self {
            text,
            time_start: None,
            active_time: expire_time,
            is_active: true,
        }
    }

    fn update(&mut self, time: f64) {
        match self.time_start {
            None => {
                self.time_start = Some(time);
            }
            Some(time_start) => {
                if time - time_start >= self.active_time {
                    self.is_active = false;
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

pub(crate) struct SideFeed {
    events: Vec<FeedElement>,
    display_time: f64,
    max_display: u8,
}

impl SideFeed {
    pub fn new(display_time: f64, max_display: u8) -> Self {
        Self {
            events: Vec::new(),
            display_time,
            max_display,
        }
    }

    pub fn add(&mut self, text: String) {
        self.events.push(FeedElement::new(text, self.display_time));
    }

    pub fn draw(&self) {
        let x = 40.;
        let mut layout = Layout::new(40., 6.);

        let text = Text::new(
            TextParams {
                font_size: TEXT_SMALL,
                ..default_text_params()
            },
            TextVerticalPositioning::CenterConsistent,
            TextHorizontalPositioning::Left,
        );

        for el in self.events.iter().take(self.max_display as usize) {
            text.draw(&el.text, x, layout.next());
            layout.add(10.);
        }
    }

    pub fn update(&mut self) {
        // Only time the ones that are displayed
        for el in self.events.iter_mut().take(self.max_display as usize) {
            el.update(macroquad::time::get_time());
        }

        self.events.retain(|el| el.is_active());
    }
}
