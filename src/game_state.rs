use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::Serialize;
use tracing::info;
use crate::wikipedia::{get_pretty_extract, get_random_article_extract};

#[derive(Serialize, Debug, Clone)]
pub struct Game {
    pub text: String,
    pub user_text: HashMap<String, UserText>
}

#[derive(Serialize, Debug, Clone)]
pub struct UserText {
    pub user_name: String,
    pub text: String
}

pub type RoomStore = HashMap<String, Game>;

#[derive(Default)]
pub struct GameStore {
    pub games: RwLock<RoomStore>
}

impl GameStore {
    pub async fn init_game(&self, room: String) {
        let mut binding = self.games.write().await;
        
        if binding.contains_key(&room) {
            return;
        }

        let wikipedia_response = get_random_article_extract().await.unwrap();

        let mut length = wikipedia_response.value.chars().count();
        for i in 0..wikipedia_response.value.chars().count() {
            if wikipedia_response.value.chars().nth(i).unwrap() == '.' && i >= 500 {
                length = i + 1;
                break;
            }
        }

        let extract = get_pretty_extract(wikipedia_response.value[..length].parse().unwrap());
        
        let game = Game {
            text: extract,
            user_text: HashMap::new()
        };

        binding.insert(room, game);
    }

    pub async fn add_user(&self, user_id: String, user_name: String, room: &String) {
        let mut binding = self.games.write().await;
        
        if let Some(game) = binding.get_mut(room) {
            game.user_text.insert(user_id, UserText { user_name, text: String::new() });
        } else { 
            panic!()
        }
    }
    
    pub async fn remove_user(&self, room: &String, user_id: &String) {
        let mut binding = self.games.write().await;
        
        if let Some(game) = binding.get_mut(room) {
            game.user_text.remove(user_id);
            if game.user_text.is_empty() {
                self.delete_game(room).await;
            }
        } else {
            panic!()
        }
    }

    pub async fn add_character(&self, room: &String, user_id: &String, character: char, index: usize) {
        let mut binding = self.games.write().await;
        let user_text = binding.get_mut(room).unwrap().user_text.get_mut(user_id).unwrap();
        
        if index >= user_text.text.len() {
            user_text.text.push(character);
        } else {
            user_text.text.insert(index, character);
        }
    }

    pub async fn pop_character(&self, room: &String, user_id: &String, index: usize) {
        let mut binding = self.games.write().await;
        let user_text = binding.get_mut(room).unwrap().user_text.get_mut(user_id).unwrap();

        user_text.text.replace_range(
            user_text.text
                .char_indices()
                .nth(index)
                .map(|(pos, char)| pos..pos + char.len_utf8())
                .unwrap(),
            ""
        );
    }

    pub async fn get_characters(&self, user_id: &String, room: &String) -> Option<UserText> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.get(user_id).cloned()
    }

    pub async fn get_users(&self, room: &String) -> Vec<String> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.keys().cloned().collect()
    }
    
    pub async fn get_user_text_all(&self, room: &String) -> HashMap<String, UserText> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.clone()
    }
    
    pub async fn delete_game(&self, room: &String) {
        let mut binding = self.games.write().await;
        binding.remove(room);
        info!("The game {} was deleted", room);
    }
    
    pub async fn get_game_text(&self, room: &String) -> String {
        let binding = self.games.read().await;
        binding.get(room).unwrap().text.clone()
    }
}