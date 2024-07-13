use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub room: String
}

pub type RoomStore = HashMap<String, User>;

#[derive(Default)]
pub struct UserStore {
    pub users: RwLock<RoomStore>
}

impl UserStore {
    pub async fn add_user(&self, user_id: String, user: User) {
        let mut binding = self.users.write().await;
        binding.insert(user_id, user);
    } 
    
    pub async fn get_user(&self, user_id: &String) -> Option<User> {
        let binding = self.users.read().await;
        binding.get(user_id).cloned()
    }
    
    pub async fn contains_user(&self, user_id: &String) -> bool {
        let binding = self.users.read().await;
        binding.contains_key(user_id)
    }
    
    pub async fn get_all_users(&self) -> HashMap<String, String> {
        let binding = self.users.read().await;
        binding.iter().map(|(user_id, user)| (user_id.clone(), user.name.clone())).collect()
    }
    
    pub async fn remove_user(&self, user_id: &String) -> Option<User> {
        let mut binding = self.users.write().await;
        binding.remove(user_id)
    }
}