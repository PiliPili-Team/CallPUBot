use teloxide::types::{ChatId, User};

#[derive(Clone)]
pub struct UserRegister {
    pub register: Option<User>,
    pub user: User,
}

pub struct CallMap(std::collections::HashMap<ChatId, Vec<UserRegister>>);

impl Default for CallMap {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CallResult {
    AlreadyRegistered,
    Registered,
}

pub enum LeaveResult {
    NotRegistered,
    Left,
}

impl CallMap {
    pub fn new() -> Self {
        Self(std::collections::HashMap::new())
    }

    pub fn register(&mut self, chat_id: ChatId, user: UserRegister) -> CallResult {
        let entry = self.0.entry(chat_id).or_default();
        if !entry.iter().any(|u| u.user.id == user.user.id) {
            entry.push(user);
            CallResult::Registered
        } else {
            CallResult::AlreadyRegistered
        }
    }

    pub fn leave(&mut self, chat_id: ChatId, this_user: User) -> LeaveResult {
        let Some(entry) = self.0.get_mut(&chat_id) else {
            return LeaveResult::NotRegistered;
        };

        let before = entry.len();
        entry.retain(|u| u.user.id != this_user.id);

        if entry.len() == before {
            LeaveResult::NotRegistered
        } else {
            LeaveResult::Left
        }
    }

    pub fn has_user(&self, chat_id: &ChatId, this_user: &User) -> bool {
        self.0
            .get(&chat_id)
            .map(|users| users.iter().any(|u| u.user.id == this_user.id))
            .unwrap_or(false)
    }

    pub fn get_call_list(&self, chat_id: ChatId) -> Vec<User> {
        self.0
            .get(&chat_id)
            .map(|users| users.iter().map(|u| u.user.clone()).collect())
            .unwrap_or_default()
    }

    pub fn get_register(&self, user: User) -> Option<User> {
        self.0.values().find_map(|users| {
            users
                .iter()
                .find(|u| u.user.id == user.id)
                .and_then(|u| u.register.clone())
        })
    }
}

