use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use tracing::{info};
use crate::states::app_state::SharedAppState;
use crate::states::game_state::AppState;
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
    app_state: AppState,
    color: HashMap<String, UserColor>
}

#[derive(Serialize)]
struct UserTextChangeOut {
    user_id: String,
    text_index: usize
}

pub async fn handle_websocket_connection(socket: SocketRef) {
    info!("Socket connected: {}", socket.id);

    socket.on("join", |socket: SocketRef, Data::<User>(user), state: State<SharedAppState>| async move {
        if user.name.is_empty() {
            let _ = socket.disconnect();
            return;
        }

        info!("User: {} joined the room: {}", user.name, user.room);

        let _ = socket.leave_all();
        let _ = socket.join(user.room.clone());

        let state = state.lock().await;

        state.games.init_game(user.room.clone()).await;
        state.games.add_user(socket.id.to_string(), user.name.clone(), &user.room).await;
        state.users.add_user(socket.id.to_string(), user.clone()).await;

        let data = UserConnectData {
            user_map: state.games.get_all_users(&user.room, &state.users.get_all_users().await).await,
            correct_text_length_map: state.games.get_correct_text_length_all(&user.room).await,
            app_state: state.games.get_app_state(&user.room).await,
            color: state.games.get_all_user_color(&user.room).await
        };

        if data.app_state == AppState::GAME {
            let _ = socket.emit("start_game", state.games.get_game_text(&user.room).await);
        }

        let _ = socket.within(user.room.clone()).emit("user_connect", data);
    });

    socket.on_disconnect(|socket: SocketRef, state: State<SharedAppState>| async move {
        info!("Received on Disconnect");
        let state = state.lock().await;
        let user_id = socket.id.to_string();

        if let Some(user) = state.users.remove_user(&user_id).await {
            let game_delete = state.games.remove_user(&user.room, &user_id).await;

            if !game_delete {
                let data = UserConnectData {
                    user_map: state.games.get_all_users(&user.room, &state.users.get_all_users().await).await,
                    correct_text_length_map: state.games.get_correct_text_length_all(&user.room).await,
                    app_state: state.games.get_app_state(&user.room).await,
                    color: state.games.get_all_user_color(&user.room).await
                };

                let _ = socket.within(user.room.clone()).emit("user_connect", data);
            }
            info!("Client: {} disconnected!", socket.id.to_string());
        }
    });

    socket.on("start_game", |socket: SocketRef, Data::<User>(user), state: State<SharedAppState>| async move {
        info!("The game in the room {} was started!", user.room);

        let state = state.lock().await;
        state.games.start_game(&user.room).await;

        let _ = socket.within(user.room.clone()).emit("start_game", state.games.get_game_text(&user.room).await);
        let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_app_state(&user.room).await);
    });

    socket.on("push_character", |socket: SocketRef, Data::<char>(character), state: State<SharedAppState>| async move {
        info!("Received push_character: {}", character);

        let state = state.lock().await;
        let user_id = socket.id.to_string();
        let user = state.users.get_user(&user_id).await.unwrap();

        if let Some(text_index) = state.games.push_character(&user.room, &user_id, character).await {
            let user_text_change = UserTextChangeOut { user_id: user_id.clone(), text_index };
            let _ = socket.within(user.room.clone()).emit("character_change", user_text_change);

            if state.games.check_ending(&user.room, &user_id).await {
                let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_app_state(&user.room).await);
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
            let _ = socket.within(user.room.clone()).emit("character_change", user_text_change);
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
            let _ = socket.within(user.room.clone()).emit("app_state_change", state.games.get_app_state(&user.room).await);
        }
    });
}