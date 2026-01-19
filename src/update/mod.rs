mod check;
mod self_update;

pub use check::{show_update_notification_if_available, spawn_update_check};
pub use self_update::perform_update;
