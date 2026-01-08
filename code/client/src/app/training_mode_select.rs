use crate::app::training::{Training, TrainingMode};
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{self};
use crate::ui::{BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_WIDTH};
use burn::backend::Wgpu;

use common::rl::BotBrain;

use macroquad::prelude::*;

type ClientBackend = Wgpu;

pub(crate) struct TrainingModeSelect {
    model_name: String,
    brain: BotBrain<ClientBackend>,
    back_clicked: bool,
    chosen_training_mode: Option<TrainingMode>,
}

impl TrainingModeSelect {
    pub fn new(model_name: String, brain: BotBrain<ClientBackend>) -> Self {
        Self {
            model_name,
            brain,
            back_clicked: false,
            chosen_training_mode: None,
        }
    }
}

impl View for TrainingModeSelect {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        // Handle Back Button Logic
        if self.back_clicked {
            return Transition::Pop;
        }

        if let Some(mode) = self.chosen_training_mode.take() {
            return Transition::Push(Box::new(Training::new(self.brain.clone(), mode)));
        }

        Transition::None
    }

    fn draw(&mut self, _ctx: &AppContext, has_input: bool) {
        let x_mid = CANONICAL_SCREEN_WIDTH / 2.;
        let mut layout = ui::Layout::new(100., 30.);

        ui::Text::new_title().draw("Select Mode", x_mid, layout.next());
        layout.add(50.);

        ui::Text::new_scaled(ui::TEXT_MID).draw(
            &format!("Model: {}", self.model_name),
            x_mid,
            layout.next(),
        );
        layout.add(50.);

        if Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                BUTTON_W * 1.5,
                BUTTON_H,
                Some("Spectator (4v4)"),
                has_input,
            )
            .poll()
        {
            self.chosen_training_mode = Some(TrainingMode::Spectator);
        }
        layout.add(BUTTON_H);

        if Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                BUTTON_W * 1.5,
                BUTTON_H,
                Some("Play Solo vs 4 Bots"),
                has_input,
            )
            .poll()
        {
            self.chosen_training_mode = Some(TrainingMode::HumanVsAi);
        }
        layout.add(BUTTON_H);

        self.back_clicked = Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                BUTTON_W,
                BUTTON_H,
                Some("Back"),
                has_input,
            )
            .poll();
    }

    fn get_id(&self) -> ViewId {
        ViewId::TrainingModeSelect
    }
}
