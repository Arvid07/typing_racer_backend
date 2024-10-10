use std::cmp::min;
use std::collections::HashMap;

use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Serialize;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;

use crate::util::user_color::UserColor;

#[derive(Serialize, Debug, Clone)]
pub struct Game {
    pub text: String,
    pub started_generating_text: bool,
    pub finished_generating_text: bool,
    pub users: HashMap<String, User>,
    pub game_state: GameState,
    pub available_colors: Vec<UserColor>,
    pub followup_game_id: String
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum GameState {
    Lobby,
    GameCountdown,
    Game,
    Ending
}

#[derive(Serialize, Debug, Clone)]
pub struct User {
    name: String,
    text: String,
    correct_len: usize,
    color: UserColor
}

impl User {
    fn new(name: String, text: String, correct_len: usize, user_color: UserColor) -> Self {
        User { name, text, correct_len, color: user_color }
    }
}

pub type RoomStore = HashMap<String, Game>;

#[derive(Default)]
pub struct GameStore {
    pub games: RwLock<RoomStore>
}

pub const TEXT_SIZE: usize = 250;

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
            started_generating_text: false,
            finished_generating_text: false,
            users: HashMap::new(),
            game_state: GameState::Lobby,
            available_colors,
            followup_game_id: String::new()
        };

        binding.insert(room, game);
        true
    }
    
    pub async fn set_start_generating_text(&self, room: &String) {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().started_generating_text = true;
    }
    
    pub async fn set_game_text(&self, room: &String, text: String) {
        let mut binding = self.games.write().await;
        let game = binding.get_mut(room).unwrap();
        game.finished_generating_text = true;
        game.text = text;
    }
    
    pub async fn is_available(&self, room: &String) -> bool {
        let binding = self.games.read().await;
        if let Some(game) = binding.get(room) {
            return game.game_state != GameState::Ending;
        }
        
        false
    }
    
    pub async fn started_generating_text(&self, room: &String) -> bool {
        let binding = self.games.read().await;
        if let Some(game) = binding.get(room) {
            return game.started_generating_text;
        }

        false
    }

    pub async fn finished_generating_text(&self, room: &String) -> bool {
        let binding = self.games.read().await;
        if let Some(game) = binding.get(room) {
            return game.finished_generating_text;
        }

        false
    }

    pub async fn add_user(&self, user_id: String, user_name: String, room: &String) {
        let mut binding = self.games.write().await;

        if let Some(game) = binding.get_mut(room) {
            if game.available_colors.is_empty() {
                let mut available_colors: Vec<UserColor> = UserColor::iter().collect();
                available_colors.shuffle(&mut thread_rng());
                game.available_colors = available_colors;
            }
            
            let user = User::new(user_name, String::new(), 0, game.available_colors.pop().unwrap());
            game.users.insert(user_id, user);
        } else {
            panic!()
        }
    }

    pub async fn remove_user(&self, room: &String, user_id: &String) -> bool {
        let mut binding = self.games.write().await;

        if let Some(game) = binding.get_mut(room) {
            if let Some(user) = game.users.remove(user_id) {
                game.available_colors.push(user.color);
                
                if game.users.is_empty() {
                    binding.remove(room);
                    return true;
                }
            }
        }

        false
    }

    pub async fn push_character(&self, room: &String, user_id: &String, character: char) -> Option<usize> {
        if !character.is_ascii() {
            return None;
        }

        let mut binding = self.games.write().await;
        let game = binding.get_mut(room).unwrap();
        let user = game.users.get_mut(user_id).unwrap();

        if user.text.len() < game.text.len() {
            user.text.push(character);

            if user.correct_len == user.text.len() - 1 && character == game.text.chars().nth(user.text.len() - 1).unwrap() {
                user.correct_len += 1;
                return Some(user.correct_len);
            }
        }

        None
    }

    pub async fn pop_character(&self, room: &String, user_id: &String) -> Option<usize> {
        let mut binding = self.games.write().await;
        let user = binding.get_mut(room).unwrap().users.get_mut(user_id).unwrap();

        user.text.pop();

        if user.correct_len > user.text.len() {
            user.correct_len -= 1;
            return Some(user.correct_len);
        }

        None
    }
    
    pub async fn get_game_text(&self, room: &String) -> String {
        let binding = self.games.read().await;
        binding.get(room).unwrap().text.clone()
    }

    pub async fn get_correct_len_all(&self, room: &String) -> HashMap<String, usize> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().users.iter().map(|(user_id, user)| (user_id.clone(), user.correct_len)).collect()
    }

    pub async fn calculate_correct_text_length_all(&self, room: &String) -> HashMap<String, usize> {
        let binding = self.games.read().await;

        let game = binding.get(room).unwrap();
        let game_text_chars: Vec<char> = game.text.chars().collect();
        let mut user_correct_length = HashMap::with_capacity(game.users.len());

        for (user_id, user) in game.users.iter() {
            let mut length = 0;
            let user_text_chars: Vec<char> = user.text.chars().collect();

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

    pub async fn get_all_users(&self, room: &String) -> HashMap<String, String> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().users.iter().map(|(user_id, user)| (user_id.clone(), user.name.clone())).collect()
    }

    pub async fn get_all_user_color(&self, room: &String) -> HashMap<String, UserColor> {
        let binding = self.games.read().await;
        binding.get(room).unwrap().users.iter().map(|(user_id, user)| (user_id.clone(), user.color)).collect()
    }

    pub async fn get_game_state(&self, room: &String) -> GameState {
        let binding = self.games.read().await;
        binding.get(room).unwrap().game_state.clone()
    }

    pub async fn start_game(&self, room: &String) {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().game_state = GameState::Game;
    }
    
    pub async fn start_game_countdown(&self, room: &String) {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().game_state = GameState::GameCountdown;
    }

    pub async fn check_ending(&self, room: &String, user_id: &String) -> bool {
        let mut binding = self.games.write().await;
        let game_text_length = binding.get(room).unwrap().text.len();
        let user_correct_len = binding.get(room).unwrap().users.get(user_id).unwrap().correct_len;

        if user_correct_len >= game_text_length {
            binding.get_mut(room).unwrap().game_state = GameState::Ending;
            return true;
        }
        
        false
    }
    
    pub async fn set_followup_game_id(&self, room: &String, game_id: String) {
        let mut binding = self.games.write().await;
        binding.get_mut(room).unwrap().followup_game_id = game_id;
    }

    pub async fn get_followup_game_id(&self, room: &String) -> String {
        let binding = self.games.read().await;
        binding.get(room).unwrap().followup_game_id.clone()
    }
}