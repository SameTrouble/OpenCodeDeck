use tauri::State;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn get_opencode_config(state: State<'_, AppState>) -> AppResult<serde_json::Value> {
    state.opencode_store.load()
}

#[tauri::command]
pub fn save_opencode_config(
    config: serde_json::Value,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.opencode_store.save(&config)
}
