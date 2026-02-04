use teloxide::types::{ChatId, User, UserId};

#[derive(Clone)]
pub struct UserRegister {
    pub register: Option<User>,
    pub user: User,
}

pub struct CallMap(std::collections::HashMap<ChatId, CallMapInner>);

type CaptchaAnswer = bool;
type CaptchaTimeout = std::time::Instant;

#[derive(Clone, Default)]
pub struct CallMapInner {
    pub user_register_list: Vec<UserRegister>,
    pub blacklist: Vec<UserId>,
    pub waiting_captcha: Vec<(UserId, CaptchaAnswer, CaptchaTimeout)>,
}

impl Default for CallMap {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CallResult {
    AlreadyRegistered,
    Registered,
    InBlacklist,
}

pub enum LeaveResult {
    NotRegistered,
    Left,
}

pub enum BlacklistResult {
    AlreadyBlacklisted,
    Blacklisted,
}

pub enum UnblacklistResult {
    NotInBlacklist,
    Unblacklisted,
}

impl CallMap {
    pub fn new() -> Self {
        Self(std::collections::HashMap::new())
    }

    pub fn register(&mut self, chat_id: ChatId, user: UserRegister) -> CallResult {
        let entry = self.0.entry(chat_id).or_default();

        if entry.blacklist.contains(&user.user.id) {
            return CallResult::InBlacklist;
        }

        if !entry
            .user_register_list
            .iter()
            .any(|u| u.user.id == user.user.id)
        {
            entry.user_register_list.push(user);
            CallResult::Registered
        } else {
            CallResult::AlreadyRegistered
        }
    }

    pub fn leave(&mut self, chat_id: ChatId, this_user: User) -> LeaveResult {
        let Some(entry) = self.0.get_mut(&chat_id) else {
            return LeaveResult::NotRegistered;
        };

        let before = entry.user_register_list.len();
        entry
            .user_register_list
            .retain(|u| u.user.id != this_user.id);

        if entry.user_register_list.len() == before {
            LeaveResult::NotRegistered
        } else {
            LeaveResult::Left
        }
    }

    pub fn has_user(&self, chat_id: &ChatId, this_user: &User) -> bool {
        self.0
            .get(chat_id)
            .map(|users| {
                users
                    .user_register_list
                    .iter()
                    .any(|u| u.user.id == this_user.id)
            })
            .unwrap_or(false)
    }

    pub fn get_call_list(&self, chat_id: ChatId) -> Vec<User> {
        self.0
            .get(&chat_id)
            .map(|users| {
                users
                    .user_register_list
                    .iter()
                    .map(|u| u.user.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_register(&self, chat_id: &ChatId, user: User) -> Option<User> {
        self.0.get(chat_id).and_then(|users| {
            users
                .user_register_list
                .iter()
                .find(|u| u.user.id == user.id)
                .and_then(|u| u.register.clone())
        })
    }

    pub fn blacklist(&mut self, chat_id: ChatId, user_id: UserId) -> BlacklistResult {
        let entry = self.0.entry(chat_id).or_default();
        if !entry.blacklist.contains(&user_id) {
            entry.blacklist.push(user_id);
            BlacklistResult::Blacklisted
        } else {
            BlacklistResult::AlreadyBlacklisted
        }
    }

    pub fn unblacklist(&mut self, chat_id: ChatId, user_id: UserId) -> UnblacklistResult {
        let Some(entry) = self.0.get_mut(&chat_id) else {
            return UnblacklistResult::NotInBlacklist;
        };

        let before = entry.blacklist.len();
        entry.blacklist.retain(|&u| u != user_id);

        if entry.blacklist.len() == before {
            UnblacklistResult::NotInBlacklist
        } else {
            UnblacklistResult::Unblacklisted
        }
    }

    pub fn is_blacklisted(&self, chat_id: &ChatId, user_id: &UserId) -> bool {
        self.0
            .get(chat_id)
            .map(|entry| entry.blacklist.contains(user_id))
            .unwrap_or(false)
    }

    pub fn has_captcha(&self, chat_id: &ChatId, user_id: &UserId) -> bool {
        self.0
            .get(chat_id)
            .map(|entry| {
                entry
                    .waiting_captcha
                    .iter()
                    .any(|(uid, _, _)| uid == user_id)
            })
            .unwrap_or(false)
    }

    pub fn push_captcha(&mut self, chat_id: ChatId, user_id: UserId, answer: CaptchaAnswer) {
        let entry = self.0.entry(chat_id).or_default();
        entry.waiting_captcha.push((user_id, answer, std::time::Instant::now() + std::time::Duration::from_secs(30)));
    }

    pub fn pop_captcha(&mut self, chat_id: ChatId, user_id: &UserId) -> Option<CaptchaAnswer> {
        let entry = self.0.get_mut(&chat_id)?;

        let now = std::time::Instant::now();
        entry.waiting_captcha.retain(|(_, _, timeout)| *timeout > now);

        if let Some(pos) = entry
            .waiting_captcha
            .iter()
            .position(|(uid, _, _)| uid == user_id)
        {
            let (_, answer, _) = entry.waiting_captcha.remove(pos);
            return Some(answer);
        }

        None
    }
}
