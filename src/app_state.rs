use std::sync::Arc;
use tokio::sync::Mutex;
use crate::game_state::GameStore;
use crate::user_state::UserStore;

#[derive(Default)]
pub struct AppState {
    pub games: GameStore,
    pub users: UserStore
}

pub type SharedAppState = Arc<Mutex<AppState>>;
