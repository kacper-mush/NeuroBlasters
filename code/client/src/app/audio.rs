use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};

pub struct AudioManager {
    background_music: Sound,
    laser_sound: Sound,
    elimination_sound: Sound,
    is_music_playing: bool,
    current_volume: f32,
    target_volume: f32,
}

impl AudioManager {
    pub async fn new() -> Self {
        let background_music = load_sound("assets/sounds/background.ogg").await.expect("Failed to load background music");
        let laser_sound = load_sound("assets/sounds/laser.ogg").await.expect("Failed to load laser sound");
        let elimination_sound = load_sound("assets/sounds/elimination.ogg").await.expect("Failed to load elimination sound");

        Self {
            background_music,
            laser_sound,
            elimination_sound,
            is_music_playing: false,
            current_volume: 1.0,
            target_volume: 1.0,
        }
    }

    pub fn start_music(&mut self) {
        if !self.is_music_playing {
            play_sound(
                &self.background_music,
                PlaySoundParams {
                    looped: true,
                    volume: self.current_volume,
                },
            );
            self.is_music_playing = true;
        }
    }

    pub fn stop_music(&mut self) {
        if self.is_music_playing {
            stop_sound(&self.background_music);
            self.is_music_playing = false;
        }
    }

    // Since macroquad 0.4 doesn't support changing volume of playing sound easily without keeping a handle (which play_sound doesn't return in some versions),
    // and if we want to change volume, we might have to restart. 
    // However, checking if we can just re-call play_sound? No, that overlaps.
    // We'll implement a simple restart-if-changed for now, or assume we can just manage two states.
    pub fn set_music_volume(&mut self, volume: f32) {
        if (self.current_volume - volume).abs() > 0.01 {
            self.current_volume = volume;
            if self.is_music_playing {
                // Restart with new volume
                // This is not ideal as it resets the track, but without a mixer/handle it's the only way in basic macroquad
                stop_sound(&self.background_music);
                play_sound(
                    &self.background_music,
                    PlaySoundParams {
                        looped: true,
                        volume: self.current_volume,
                    },
                );
            }
        }
    }

    pub fn play_shot(&self) {
        play_sound(
            &self.laser_sound,
            PlaySoundParams {
                looped: false,
                volume: 0.5,
            },
        );
    }

    pub fn play_elimination(&self) {
        play_sound(
            &self.elimination_sound,
            PlaySoundParams {
                looped: false,
                volume: 0.8,
            },
        );
    }
}
