use super::MouseState;
use crate::project::Track;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub(super) struct InputState {
    pub(super) control_pressed: bool,
    pub(super) shift_pressed: bool,
    pub(super) mouse: MouseState,
    pub(super) right_mouse_held: bool,
    pub(super) last_click: Option<std::time::Instant>,
}

#[derive(Default)]
pub(super) struct FileOperations {
    pub(super) track_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub(super) project_open_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub(super) project_save_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pub(super) track_load_rx: Option<Receiver<Track>>,
    pub(super) pending_drop: Option<(u32, usize)>,
}

#[derive(Default)]
pub(super) struct ShutdownState {
    pub(super) close_requested: bool,
    pub(super) closing: bool,
    pub(super) close_save_submitted: bool,
}
