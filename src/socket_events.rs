use std::collections::HashMap;
use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use tokio::sync::MutexGuard;
use tokio::time::sleep;
use tracing::{info};
use uuid::Uuid;
use crate::states::app_state::{AppState, SharedAppState};
use crate::states::game_state::GameState;
use crate::states::user_state::User;
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
    color: HashMap<String, UserColor>
}

#[derive(Serialize)]
struct UserTextChangeOut {
    user_id: String,
    text_index: usize
}

async fn user_join<'a>(socket: &SocketRef, user: User, state: MutexGuard<'a, AppState>) {
    let _ = socket.leave_all();
    let _ = socket.join(user.room.clone());

    let game_create = state.games.init_game(user.room.clone()).await;
    state.games.add_user(socket.id.to_string(), user.name.clone(), &user.room).await;

    let data = UserConnectData {
        user_map: state.games.get_all_users(&user.room, &state.users.get_all_users().await).await,
        correct_text_length_map: state.games.get_correct_text_length_all(&user.room).await,
        app_state: state.games.get_game_state(&user.room).await,
        color: state.games.get_all_user_color(&user.room).await
    };

    if data.app_state == GameState::GAME {
        let _ = socket.emit("start_game", state.games.get_game_text(&user.room).await);
    }

    let _ = socket.within(user.room.clone()).emit("user_connect", data);
    
    if game_create {
        state.games.generate_text(&user.room).await;
    }
}

async fn user_leave<'a>(socket: &SocketRef, state: MutexGuard<'a, AppState>) {
    let user_id = socket.id.to_string();

    if let Some(user) = state.users.remove_user(&user_id).await {
        let game_delete = state.games.remove_user(&user.room, &user_id).await;

        if !game_delete {
            let data = UserConnectData {
                user_map: state.games.get_all_users(&user.room, &state.users.get_all_users().await).await,
                correct_text_length_map: state.games.get_correct_text_length_all(&user.room).await,
                app_state: state.games.get_game_state(&user.room).await,
                color: state.games.get_all_user_color(&user.room).await
            };

            let _ = socket.within(user.room.clone()).emit("user_connect", data);
        }
        info!("Client: {} left!", socket.id.to_string());
    }
}

pub async fn handle_websocket_connection(socket: SocketRef) {
    info!("Socket connected: {}", socket.id);

    socket.on("join_game", |socket: SocketRef, Data::<User>(user), state: State<SharedAppState>| async move {
        if user.name.is_empty() {
            return;
        }
        
        let state = state.lock().await;
        if !state.games.is_available(&user.room).await {
            let _ = socket.emit("game_unavailable", "");
            return;
        }

        info!("User: {} joined the room: {}", user.name, user.room);
        state.users.add_user(socket.id.to_string(), user.clone()).await;
        user_join(&socket, user, state).await;
        
        let _ = socket.emit("allowed_to_join", "");
    });

    socket.on("create_game", |socket: SocketRef, Data::<User>(mut user), state: State<SharedAppState>| async move {
        if user.name.is_empty() {
            return;
        }
        
        let state = state.lock().await;
        if state.users.contains_user(&socket.id.to_string()).await {
            return;
        }
        
        let game_id = Uuid::new_v4();
        user.room.clone_from(&game_id.to_string());
        
        state.users.add_user(socket.id.to_string(), user.clone()).await;

        info!("User: {} created the room: {}", user.name, user.room);
        user_join(&socket, user, state).await;
        
        let _ = socket.emit("game_id", game_id.to_string());
    });

    socket.on_disconnect(|socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received on Disconnect");
        let state = state.lock().await;
        user_leave(&socket, state).await;
    });
    
    socket.on("leave_game", |socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received leave_game");
        let state = state.lock().await;
        user_leave(&socket, state).await;
    });

    socket.on("start_game", |socket: SocketRef, Data::<User>(user), state: State<SharedAppState>| async move {        
        info!("The game in the room {} was started!", user.room);

        let state = state.lock().await;
        if state.games.get_game_state(&user.room).await != GameState::LOBBY {
            return;
        }
        
        state.games.start_game(&user.room).await;
        let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_game_state(&user.room).await);
        let game_text = state.games.get_game_text(&user.room).await;
        
        tokio::spawn(async move {
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

            let _ = socket.within(user.room.clone()).emit("start_game", game_text);
        });
    });

    socket.on("push_character", |socket: SocketRef, Data::<char>(character), state: State<SharedAppState>| async move {
        info!("Received push_character: {}", character);

        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        if let Some(text_index) = state.games.push_character(&user.room, &user_id, character).await {
            let user_text_change = UserTextChangeOut { user_id: user_id.clone(), text_index };
            let _ = socket.within(user.room.clone()).broadcast().emit("character_change", user_text_change);

            if state.games.check_ending(&user.room, &user_id).await {
                let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_game_state(&user.room).await);
            }
        }
    });

    socket.on("pop_character", |socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received pop_character");

        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        if let Some(text_index) = state.games.pop_character(&user.room, &user_id).await {
            let user_text_change = UserTextChangeOut { user_id, text_index };
            let _ = socket.within(user.room.clone()).broadcast().emit("character_change", user_text_change);
            info!("emitted character_change");
        }
    });

    socket.on("text_insert", |socket: SocketRef, Data::<String>(text), state: State<SharedAppState>| async move {
        info!("Received text_insert");

        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        let text_index = state.games.insert_user_text(&user.room, &user_id, text).await;
        let data = UserTextChangeOut { user_id: user_id.clone(), text_index };

        let _ = socket.within(user.room.clone()).emit("text_change", data);

        if state.games.check_ending(&user.room, &user_id).await {
            let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_game_state(&user.room).await);
        }
    });
    
    socket.on("check_game_availability", |socket: SocketRef, Data::<String>(room), state: State<SharedAppState>| async move {
        info!("Received check_game_availability");
        let state = state.lock().await;
        
        match state.games.is_available(&room).await {
            true => {
                let _ = socket.emit("game_available", "");
            }
            false => {
                let _ = socket.emit("game_unavailable", "");
            }
        }
    });
}