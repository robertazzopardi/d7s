use crate::app_state::AppState;

#[must_use]
pub fn default_idle_hint(app_state: AppState) -> String {
    match app_state {
        AppState::ConnectionList => "? help · n new · q quit".to_string(),
        AppState::DatabaseConnected => "? help · Esc back · q quit".to_string(),
    }
}
