use super::{App, State};
use crate::audio::AudioCommand;
use crate::graphics::primitives::RenameTarget;
use ringbuf::traits::Producer;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

impl App {
    /// Handles one keyboard event while a RenameState is active.
    /// Returns true if the event was consumed (caller should not fall through
    /// to normal keybind handling).
    pub(super) fn handle_rename_key(&mut self, event: &KeyEvent) -> bool {
        let State::Ready(gfx) = &mut self.state else {
            return false;
        };
        let Some(renaming) = &mut gfx.renaming else {
            return false;
        };

        match event.physical_key {
            PhysicalKey::Code(KeyCode::Enter) => {
                let final_name = renaming.edited_name.clone();
                match renaming.target {
                    RenameTarget::Pattern(id) => {
                        self.producer
                            .try_push(AudioCommand::RenamePattern(id, final_name))
                            .ok();
                    }
                    RenameTarget::Track(id) => {
                        self.producer
                            .try_push(AudioCommand::RenameTrack(id, final_name))
                            .ok();
                    }
                }
                self.project_is_dirty = true;
                gfx.renaming = None;
            }
            PhysicalKey::Code(KeyCode::Escape) => gfx.renaming = None,
            PhysicalKey::Code(KeyCode::Backspace) => {
                if renaming.cursor > 0 {
                    renaming.edited_name.remove(renaming.cursor - 1);
                    renaming.cursor -= 1;
                }
            }
            PhysicalKey::Code(KeyCode::Delete) => {
                if renaming.cursor < renaming.edited_name.len() {
                    renaming.edited_name.remove(renaming.cursor);
                }
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                renaming.cursor = renaming.cursor.saturating_sub(1);
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                renaming.cursor = (renaming.cursor + 1).min(renaming.edited_name.len());
            }
            _ => {
                if let Some(text) = &event.text {
                    renaming
                        .edited_name
                        .insert_str(renaming.cursor, text.as_str());
                    renaming.cursor += text.len();
                }
            }
        }
        true
    }
}
