mod tray;
mod window;

pub use tray::setup_tray;
pub use window::{
    apply_always_on_top, apply_click_through, apply_widget_mode, current_window_state,
    restore_window_preferences,
};
