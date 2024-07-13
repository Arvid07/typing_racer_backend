use std::cmp::min;
use std::collections::{HashMap};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tokio::sync::RwLock;
use tracing::info;
use crate::text::wikipedia::{get_pretty_extract, get_random_article_extract};
use crate::util::user_color::UserColor;

#[derive(Serialize, Debug, Clone)]
pub struct Game {
    pub text: String,
    pub user_text: HashMap<String, UserText>,
    pub correct_text_length: HashMap<String, usize>,
    pub user_color: HashMap<String, UserColor>,
    pub game_state: GameState,
    pub available_colors: Vec<UserColor>
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum GameState {
    LOBBY,
    GAME,
    ENDING
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserText {
    pub user_name: String,
    pub text: String
}

pub type RoomStore = HashMap<String, Game>;

#[derive(Default)]
pub struct GameStore {
    pub games: RwLock<RoomStore>
}

const TEXT_SIZE: usize = 1;

impl GameStore {
    pub async fn init_game(&self, room: String) -> bool {
        let mut binding = self.games.write().await;

        if binding.contains_key(&room) {
            return false;
        }

        let mut available_colors: Vec<UserColor> = UserColor::iter().collect();
        available_colors.shuffle(&mut thread_rng());

        let game = Game {
            text: String::new(),
            correct_text_length: HashMap::new(),
            user_text: HashMap::new(),
            user_color: HashMap::new(),
            game_state: GameState::LOBBY,
            available_colors
        };

        binding.insert(room, game);
        true
    }
    
    pub async fn generate_text(&self, room: &String) {
        // let mut extract = String::new();
        let mut extract = "aaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();

        while extract.chars().count() < TEXT_SIZE {
            let wikipedia_response = get_random_article_extract().await.unwrap();
            let mut length = wikipedia_response.value.chars().count();

            for i in TEXT_SIZE..wikipedia_response.value.chars().count() {
                if wikipedia_response.value.chars().nth(i).unwrap() == '.' {
                    length = i + 1;
                    break;
                }
            }

            if let Some(pretty_extract) = get_pretty_extract(wikipedia_response.value[..length].parse().unwrap()) {
                extract = pretty_extract;
            } else {
                info!("extract does contain non ascii letters!");
            }
        }

        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().text = extract;
    }
    
    pub async fn is_available(&self, room: &String) -> bool {
        let binding = self.games.read().await;
        
        if let Some(game) = binding.get(room) {
            return game.game_state != GameState::ENDING;
        }
        
        false
    }

    pub async fn add_user(&self, user_id: String, user_name: String, room: &String) {
        let mut binding = self.games.write().await;

        if let Some(game) = binding.get_mut(room) {
            game.user_text.insert(user_id.clone(), UserText { user_name, text: String::new() });
            game.correct_text_length.insert(user_id.clone(), 0);

            if game.available_colors.is_empty() {
                let mut available_colors: Vec<UserColor> = UserColor::iter().collect();
                available_colors.shuffle(&mut thread_rng());
                game.available_colors = available_colors;
            }

            game.user_color.insert(user_id, game.available_colors.pop().unwrap());
        } else {
            panic!()
        }
    }

    pub async fn remove_user(&self, room: &String, user_id: &String) -> bool {
        let mut binding = self.games.write().await;

        if let Some(game) = binding.get_mut(room) {
            game.user_text.remove(user_id);
            game.correct_text_length.remove(user_id);
            let color = game.user_color.remove(user_id).unwrap();
            game.available_colors.push(color);

            if game.user_text.is_empty() {
                binding.remove(room);
                return true;
            }
        } else {
            panic!()
        }

        false
    }

    pub async fn push_character(&self, room: &String, user_id: &String, character: char) -> Option<usize> {
        if !character.is_ascii() {
            return None;
        }

        let mut binding = self.games.write().await;
        let game = binding.get_mut(room).unwrap();
        let user_text = game.user_text.get_mut(user_id).unwrap();
        let correct_text_length = game.correct_text_length.get_mut(user_id).unwrap();

        if user_text.text.len() < game.text.len() {
            user_text.text.push(character);

            if *correct_text_length == user_text.text.len() - 1 && character == game.text.chars().nth(user_text.text.len() - 1).unwrap() {
                *correct_text_length += 1;
                return Some(*correct_text_length);
            }
        }

        None
    }

    pub async fn pop_character(&self, room: &String, user_id: &String) -> Option<usize> {
        let mut binding = self.games.write().await;
        let game = binding.get_mut(room).unwrap();
        let user_text = game.user_text.get_mut(user_id).unwrap();
        let correct_text_length = game.correct_text_length.get_mut(user_id).unwrap();

        user_text.text.pop();

        if *correct_text_length > user_text.text.len() {
            *correct_text_length -= 1;
            return Some(*correct_text_length);
        }

        None
    }

    pub async fn get_characters(&self, user_id: &String, room: &String) -> Option<UserText> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.get(user_id).cloned()
    }

    pub async fn get_users(&self, room: &String) -> Vec<String> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.keys().cloned().collect()
    }

    pub async fn get_user_text(&self, room: &String, user_id: &String) -> String {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.get(user_id).unwrap().text.clone()
    }

    pub async fn get_user_text_all(&self, room: &String) -> HashMap<String, UserText> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.clone()
    }

    pub async fn get_game_text(&self, room: &String) -> String {
        let binding = self.games.read().await;
        binding.get(room).unwrap().text.clone()
    }

    pub async fn get_correct_text_length_all(&self, room: &String) -> HashMap<String, usize> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().correct_text_length.clone()
    }

    pub async fn calculate_correct_text_length_all(&self, room: &String) -> HashMap<String, usize> {
        let binding = self.games.read().await;

        let game = binding.get(room).unwrap();
        let game_text_chars: Vec<char> = game.text.chars().collect();
        let mut user_correct_length = HashMap::with_capacity(game.user_text.len());

        for (user_id, user_text) in game.user_text.iter() {
            let mut length = 0;
            let user_text_chars: Vec<char> = user_text.text.chars().collect();

            for i in 0..min(game_text_chars.len(), user_text_chars.len()) {
                if game_text_chars[i] != user_text_chars[i] {
                    break;
                }
                length = i + 1;
            }

            user_correct_length.insert(user_id.clone(), length);
        }

        user_correct_length
    }

    pub async fn insert_user_text(&self, room: &String, user_id: &String, user_text: String) -> usize {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().user_text.get_mut(user_id).unwrap().text = user_text;

        let game_text: Vec<char> = binding.get(room).unwrap().text.chars().collect();
        let user_text: Vec<char> = binding.get(room).unwrap().user_text.get(user_id).unwrap().text.chars().collect();

        let mut length = 0;
        for i in 0..min(game_text.len(), user_text.len()) {
            if game_text[i] != user_text[i] {
                break;
            }
            length = i + 1;
        }

        binding.get_mut(room).unwrap().correct_text_length.insert(user_id.clone(), length);

        length
    }

    pub async fn get_all_users(&self, room: &String, user_map: &HashMap<String, String>) -> HashMap<String, String> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_text.keys().map(|user_id| (user_id.clone(), user_map.get(user_id).unwrap().clone())).collect()
    }

    pub async fn get_all_user_color(&self, room: &String) -> HashMap<String, UserColor> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().user_color.clone()
    }

    pub async fn get_game_state(&self, room: &String) -> GameState {
        let binding = self.games.read().await;
        binding.get(room).unwrap().game_state.clone()
    }

    pub async fn start_game(&self, room: &String) {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().game_state = GameState::GAME;
    }

    pub async fn check_ending(&self, room: &String, user_id: &String) -> bool {
        let mut binding = self.games.write().await;
        let game_text_length = binding.get(room).unwrap().text.len();
        let user_text_index = binding.get(room).unwrap().correct_text_length.get(user_id).unwrap();

        if user_text_index >= &game_text_length {
            binding.get_mut(room).unwrap().game_state = GameState::ENDING;
            return true;
        }
        
        false
    }
}