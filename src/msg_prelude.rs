use teloxide::{types::User, utils::html};

pub trait ReplaceUserExt {
    fn replace_user(&self, user: User) -> String;
}

impl<T> ReplaceUserExt for T
where
    T: ToString,
{
    fn replace_user(&self, user: User) -> String {
        self.to_string().replace(
            "#User#",
            &html::user_mention(user.id, user.full_name().as_str()),
        )
    }
}
