use std::collections::HashMap;
use std::ops::{Sub};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use tokio::sync::MutexGuard;
use tokio::time::sleep;
use tracing::{info};
use uuid::Uuid;
use crate::states::app_state::{AppState, SharedAppState};
use crate::states::game_state::{GameState, TEXT_SIZE};
use crate::states::user_state::UserInfo;
use crate::text::wikipedia::{get_pretty_extract, get_random_article_extract};
use crate::util::user_color::UserColor;

#[derive(Debug, Serialize, Deserialize)]
struct Character {
    character: char,
    index: usize
}

#[derive(Debug, Serialize)]
struct UserConnectData {
    user_map: HashMap<String, String>,
    correct_text_length_map: HashMap<String, usize>,
    app_state: GameState,
    color: HashMap<String, UserColor>,
    finished_generating_text: bool
}

#[derive(Serialize)]
struct UserTextChangeOut {
    user_id: String,
    text_index: usize
}

async fn user_join<'a>(socket: &SocketRef, user: &UserInfo, state: &MutexGuard<'a, AppState>) -> bool {
    state.users.add_user(socket.id.to_string(), user.clone()).await;
    
    let _ = socket.leave_all();
    let _ = socket.join(user.room.clone());

    let game_create = state.games.init_game(user.room.clone()).await;
    state.games.add_user(socket.id.to_string(), user.name.clone(), &user.room).await;

    let data = get_game_data(&user.room, state).await;

    if data.app_state == GameState::Game {
        let _ = socket.emit("start_game", state.games.get_game_text(&user.room).await);
    }
    
    let _ = socket.within(user.room.clone()).emit("user_connect", data);
    info!("Client: {} joined!", socket.id.to_string());
    game_create
}

async fn user_leave<'a>(socket: &SocketRef, state: &MutexGuard<'a, AppState>) {
    let user_id = socket.id.to_string();

    if let Some(user) = state.users.remove_user(&user_id).await {
        let game_delete = state.games.remove_user(&user.room, &user_id).await;

        if !game_delete {
            let _ = socket.within(user.room.clone()).emit("user_connect", get_game_data(&user.room, state).await);
        }
        info!("Client: {} left!", socket.id.to_string());
    }
}

async fn create_game<'a>(socket: &SocketRef, state: &MutexGuard<'a, AppState>, game_id: String, user: &mut UserInfo) {
    user.room.clone_from(&game_id);
    let room = user.room.clone();

    info!("User: {} created the room: {}", user.name, room);
    user_join(socket, user, state).await;

    let _ = socket.emit("game_id", game_id);
}

async fn get_game_data<'a>(room: &String, state: &MutexGuard<'a, AppState>) -> UserConnectData {
    UserConnectData {
        user_map: state.games.get_all_users(room).await,
        correct_text_length_map: state.games.get_correct_len_all(room).await,
        app_state: state.games.get_game_state(room).await,
        color: state.games.get_all_user_color(room).await,
        finished_generating_text: state.games.finished_generating_text(room).await
    }
}

pub async fn handle_websocket_connection(socket: SocketRef) {
    info!("Socket connected: {}", socket.id);

    socket.on("join_game", |socket: SocketRef, Data::<UserInfo>(user), state: State<SharedAppState>| async move {
        if user.name.is_empty() {
            return;
        }
        
        let state = state.lock().await;
        if !state.games.is_available(&user.room).await {
            let _ = socket.emit("game_unavailable", "");
            return;
        }

        info!("User: {} joined the room: {}", user.name, user.room);
        user_join(&socket, &user, &state).await;
        
        let _ = socket.emit("allowed_to_join", "");
    });

    socket.on("create_game", |socket: SocketRef, Data::<UserInfo>(mut user), state: State<SharedAppState>| async move {
        if user.name.is_empty() {
            return;
        }
        
        let state = state.lock().await;
        if state.users.contains_user(&socket.id.to_string()).await {
            return;
        }
        
        let game_id = Uuid::new_v4().to_string();
        create_game(&socket, &state, game_id, &mut user).await;
    });
    
    socket.on("play_again", |socket: SocketRef, Data::<UserInfo>(mut user), state: State<SharedAppState>| async move {
        info!("Received play_again");
        let state = state.lock().await;
        let room = user.room.clone();
        
        if state.games.get_game_state(&room).await != GameState::Ending {
            return;
        }
        
        let mut game_id = state.games.get_followup_game_id(&room).await;
        
        if state.games.is_available(&game_id).await {
            let _ = socket.emit("game_id", game_id.clone());
            user.room = game_id;

            user_leave(&socket, &state).await;
            user_join(&socket, &user, &state).await;
        } else {
            game_id = Uuid::new_v4().to_string();
            state.games.set_followup_game_id(&room, game_id.clone()).await;

            user_leave(&socket, &state).await;
            create_game(&socket, &state, game_id, &mut user).await;
        }
    });
    
    socket.on("generate_game_text", |socket: SocketRef, Data::<UserInfo>(user), state: State<SharedAppState>| async move {
        let state_guard = state.lock().await;
        let room = user.room;

        if !state_guard.games.started_generating_text(&room).await {
            state_guard.games.set_start_generating_text(&room).await;
            drop(state_guard);
            
            let mut extract = String::new();
            while extract.chars().count() < TEXT_SIZE { 
                
                let wikipedia_response = get_random_article_extract().await.unwrap();
                let mut length = wikipedia_response.value.chars().count();

                for i in TEXT_SIZE..wikipedia_response.value.chars().count() {
                    if wikipedia_response.value.chars().nth(i).unwrap() == '.' {
                        length = i + 1;
                        break;
                    }
                }

                if let Some(pretty_extract) = get_pretty_extract(wikipedia_response.value[..length].to_string()) {
                    extract = pretty_extract;
                }
            }

            let state_guard = state.lock().await;
            state_guard.games.set_game_text(&room, extract).await;
        }

        let _ = socket.within(room).emit("created_game_text", true);
    });

    socket.on_disconnect(|socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received on Disconnect");
        let state = state.lock().await;
        user_leave(&socket, &state).await;
    });
    
    socket.on("leave_game", |socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received leave_game");
        let state = state.lock().await;
        user_leave(&socket, &state).await;
    });

    socket.on("start_game", |socket: SocketRef, Data::<UserInfo>(user), state: State<SharedAppState>| async move {        
        info!("The game in the room {} was started!", user.room);

        let state_guard = state.lock().await;
        
        if state_guard.games.get_game_state(&user.room).await != GameState::Lobby {
            return;
        }

        let game_text = state_guard.games.get_game_text(&user.room).await;
        if game_text.is_empty() {
            let _ = socket.within(user.room.clone()).emit("missing_game_text", false);
            return;
        }

        state_guard.games.start_game_countdown(&user.room).await;
        drop(state_guard);
        
        let _ = socket.within(user.room.clone()).emit("app_state_change", GameState::GameCountdown);

        let mut seconds = 5;
        let _ = socket.within(user.room.clone()).emit("countdown_change", seconds);

        let mut last_time = SystemTime::now().sub(Duration::from_secs(1));
        while seconds > 0 {
            let now = SystemTime::now();
            sleep(now.duration_since(last_time).unwrap()).await;

            seconds -= 1;
            let _ = socket.within(user.room.clone()).emit("countdown_change", seconds);
            last_time = now;
        }

        let _ = socket.within(user.room.clone()).emit("app_state_change", GameState::Game);
        let _ = socket.within(user.room.clone()).emit("start_game", game_text);
        
        let state_guard = state.lock().await;
        state_guard.games.start_game(&user.room).await;
    });

    socket.on("push_character", |socket: SocketRef, Data::<char>(character), state: State<SharedAppState>| async move {
        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        if let Some(text_index) = state.games.push_character(&user.room, &user_id, character).await {
            let user_text_change = UserTextChangeOut { user_id: user_id.clone(), text_index };
            let _ = socket.within(user.room.clone()).broadcast().emit("character_change", user_text_change);

            if state.games.check_ending(&user.room, &user_id).await {
                info!("The Game {} has finished", &user.room);
                let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_game_state(&user.room).await);
            }
        }
    });

    socket.on("pop_character", |socket: SocketRef, state: State<SharedAppState>| async move {
        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        if let Some(text_index) = state.games.pop_character(&user.room, &user_id).await {
            let user_text_change = UserTextChangeOut { user_id, text_index };
            let _ = socket.within(user.room.clone()).broadcast().emit("character_change", user_text_change);
        }
    });

    socket.on("check_game_availability", |socket: SocketRef, Data::<String>(room), state: State<SharedAppState>| async move {
        let state = state.lock().await;
        
        match state.games.is_available(&room).await {
            true => {
                let _ = socket.emit("game_available", true);
            }
            false => {
                let _ = socket.emit("game_unavailable", false);
            }
        }
    });
}