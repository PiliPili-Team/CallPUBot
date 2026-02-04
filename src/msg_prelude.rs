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
            &user_mention(&user)
        )
    }
}


fn user_mention(user: &User) -> String {
    if let Some(mention) = &user.mention() {
        mention.to_owned()
    } else {
        html::user_mention(user.id, user.full_name().as_str())
    }
}