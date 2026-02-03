use std::sync::Arc;

use anyhow::Ok;
use sysinfo::System;
use teloxide::{
    dispatching::UpdateFilterExt,
    payloads,
    prelude::*,
    requests::JsonRequest,
    types::{Me, Message, MessageId, Recipient, User},
    utils::command::BotCommands,
};
use teloxide::{dispatching::dialogue::GetChatId, utils::html};
use tokio::sync::Mutex;

use crate::{
    CallResult, LeaveResult, ReplaceUserExt, UserRegister,
    call_map::CallMap,
    cmd::{self, Command},
};

pub struct Bot(Arc<Mutex<BotInner>>);

impl Clone for Bot {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Bot {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(BotInner::new())))
    }

    pub async fn run_active(&self) -> anyhow::Result<()> {
        let bot_instance = {
            let inner = self.0.lock().await;
            inner.bot.clone()
        };

        let handler = dptree::entry().branch(Update::filter_message().endpoint({
            let bot = self.clone();

            move |_: teloxide::Bot, msg: Message, me: Me| {
                let bot = bot.clone();

                async move { bot.0.lock().await.handle_command(msg, me).await }
            }
        }));

        tracing::info!("Bot is running...");

        Dispatcher::builder(bot_instance, handler)
            .build()
            .dispatch()
            .await;

        Err(anyhow::anyhow!("Bot is stopped"))
    }
}

struct BotInner {
    bot: teloxide::Bot,
    callmap: CallMap,
}

type SendMessage = JsonRequest<payloads::SendMessage>;

trait SendMessageExt {
    async fn remove_later_30s(
        self, inner: &BotInner, from_msg: MessageId,
    ) -> anyhow::Result<Message>;
}

impl SendMessageExt for SendMessage {
    async fn remove_later_30s(
        self, inner: &BotInner, from_msg_id: MessageId,
    ) -> anyhow::Result<Message> {
        let bot = inner.bot.clone();

        let sent = self.send().await?;

        let recipient: Recipient = sent.chat.id.into();
        let msg_id = sent.id;

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = bot
                .delete_messages(recipient, vec![msg_id, from_msg_id])
                .send()
                .await;
        });

        Ok(sent)
    }
}

impl BotInner {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .https_only(true)
            .http2_adaptive_window(true)
            .build()
            .expect("failed to build http client");

        let bot = teloxide::Bot::with_client(crate::BOT_TOKEN, http_client);

        Self {
            bot,
            callmap: CallMap::new(),
        }
    }

    fn send_message<C, T>(&self, chat_id: C, text: T) -> SendMessage
    where
        C: Into<Recipient>,
        T: Into<String>,
    {
        self.bot.send_message(chat_id, text)
    }

    async fn handle_command(&mut self, msg: Message, me: Me) -> anyhow::Result<()> {
        tracing::debug!("Received message: {:?}", msg);

        if msg.from.is_none() {
            return Ok(());
        };

        if msg.chat_id() != Some(ChatId(crate::WHITE_GROUP)) {
            self.send_message(msg.chat.id, "请在 P游戏部 群内使用此机器人")
                .remove_later_30s(self, msg.id)
                .await?;

            return Ok(());
        }

        if let Some(cmd) = msg
            .text()
            .and_then(|msg| cmd::Command::parse(msg, me.username()).ok())
        {
            self.handle_command_inner(msg, cmd).await?
        } else {
            self.handle_message(msg).await?
        }

        Ok(())
    }

    async fn handle_message(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg.text() {
            Some("r") | Some("R") => self.register_user(msg).await?,
            Some("l") | Some("L") | Some("丨") => self.leave_user(msg).await?,
            Some("c") | Some("C") => self.call_pu(msg).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_command_inner(&mut self, msg: Message, cmd: Command) -> anyhow::Result<()> {
        match cmd {
            Command::Help => self.handle_help_request(msg).await,
            Command::CallPU => self.call_pu(msg).await,
            Command::Register => self.register_user(msg).await,
            Command::Leave => self.leave_user(msg).await,
            Command::WhoRegisteredMe => self.who_registered_me(msg).await,
        }
    }

    async fn who_registered_me(&mut self, msg: Message) -> anyhow::Result<()> {
        let chat_id = msg.chat.id;
        let Some(from_user) = msg.from else {
            return Ok(());
        };

        if !self.callmap.has_user(&chat_id, &from_user) {
            self.send_message(
                msg.chat.id,
                "#User# 还没有人注册你捏".replace_user(from_user),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .remove_later_30s(self, msg.id)
            .await?;
            return Ok(());
        }

        if let Some(registered_by) = self.callmap.get_register(from_user) {
            self.send_message(
                msg.chat.id,
                "查到了！#User# 注册了你捏".replace_user(registered_by.clone()),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .remove_later_30s(self, msg.id)
            .await?;
        } else {
            self.send_message(msg.chat.id, "10% 的几率！ Bot 忘了捏")
                .remove_later_30s(self, msg.id)
                .await?;
        }

        Ok(())
    }

    async fn call_pu(&mut self, msg: Message) -> anyhow::Result<()> {
        let chat_id = msg.chat.id;
        let Some(from_user) = msg.from else {
            return Ok(());
        };

        let call_list = self.callmap.get_call_list(chat_id);

        if call_list.is_empty() {
            self.send_message(msg.chat.id, "没有人捏，你来 r 一下吧")
                .remove_later_30s(self, msg.id)
                .await?;
            return Ok(());
        }

        let is_in_list = call_list.iter().any(|user| user.id == from_user.id);
        if !is_in_list {
            self.send_message(msg.chat.id, "你不许参加 impart !")
                .remove_later_30s(self, msg.id)
                .await?;
            return Ok(());
        }

        let mention_list = call_list
            .iter()
            .filter_map(|user: &User| {
                if from_user.id == user.id {
                    None
                } else {
                    Some(html::user_mention(user.id, user.full_name().as_str()))
                }
            })
            .collect::<Vec<_>>();

        if mention_list.is_empty() {
            self.send_message(msg.chat.id, "没有其他人捏，叫一个吧")
                .remove_later_30s(self, msg.id)
                .await?;
            return Ok(());
        }

        let mention_msg = mention_list.join("\n");

        self.send_message(msg.chat.id, format!("正在 Call PU：\n{}\n\n温馨提示：\n使用 /whoregisteredme 可以查看是谁把您拉进来的捏", mention_msg))
            .parse_mode(teloxide::types::ParseMode::Html)
            .remove_later_30s(self, msg.id)
            .await?;
        Ok(())
    }

    async fn register_user(&mut self, msg: Message) -> anyhow::Result<()> {
        let chat_id = msg.chat.id;

        let Some(ref from) = msg.from else {
            return Ok(());
        };

        if let Some(reply_to) = msg.reply_to_message() {
            if let Some(user) = &reply_to.from {
                let anonymous = (rand::random::<u32>() % 10) == 0;

                let user_register = if anonymous {
                    UserRegister {
                        register: None,
                        user: user.clone(),
                    }
                } else {
                    UserRegister {
                        register: Some(from.clone()),
                        user: user.clone(),
                    }
                };

                match self.callmap.register(chat_id, user_register) {
                    CallResult::AlreadyRegistered => {
                        self.send_message(msg.chat.id, "该用户已经注册过了！")
                            .remove_later_30s(self, msg.id)
                            .await?
                    }
                    CallResult::Registered => {
                        self.send_message(
                            msg.chat.id,
                            "注册成功！#User# 现在会被 Call 了".replace_user(user.clone()),
                        )
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .remove_later_30s(self, msg.id)
                        .await?
                    }
                };
                return Ok(());
            }
        }

        let from = UserRegister {
            register: Some(from.clone()),
            user: from.clone(),
        };

        match self.callmap.register(chat_id, from.clone()) {
            CallResult::AlreadyRegistered => {
                self.send_message(msg.chat.id, "你已经注册过了！")
                    .remove_later_30s(self, msg.id)
                    .await?
            }
            CallResult::Registered => {
                self.send_message(
                    msg.chat.id,
                    "注册成功！#User# 现在会被 Call 了".replace_user(from.user),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .remove_later_30s(self, msg.id)
                .await?
            }
        };

        Ok(())
    }

    async fn leave_user(&mut self, msg: Message) -> anyhow::Result<()> {
        let chat_id = msg.chat.id;
        let Some(user) = msg.from else {
            return Ok(());
        };

        match self.callmap.leave(chat_id, user.clone()) {
            LeaveResult::NotRegistered => {
                self.send_message(msg.chat.id, "你还没有注册过！")
                    .remove_later_30s(self, msg.id)
                    .await?
            }
            LeaveResult::Left => {
                self.send_message(msg.chat.id, "#User# 已离开被 Call 列表".replace_user(user))
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .remove_later_30s(self, msg.id)
                    .await?
            }
        };

        Ok(())
    }

    async fn handle_help_request(&self, msg: Message) -> anyhow::Result<()> {
        let cmd_descriptions = Command::descriptions().to_string();

        let sys_status = sys_status();

        let help_msg = format!("{}\n\n{}", cmd_descriptions, sys_status);

        self.send_message(msg.chat.id, help_msg)
            .remove_later_30s(self, msg.id)
            .await?;

        tracing::info!("send help done");
        Ok(())
    }
}

fn sys_status() -> String {
    let sys = sysinfo::System::new_all();
    let mem_usage = format!(
        "{} / {} MB",
        sys.used_memory() / 1024 / 1024,
        sys.total_memory() / 1024 / 1024
    );

    let swap_usage = format!(
        "{} / {} MB",
        sys.used_swap() / 1024 / 1024,
        sys.total_swap() / 1024 / 1024
    );

    let cpu_usage = format!("{:.2} %", sys.global_cpu_usage());

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk_usage = disks
        .iter()
        .map(|disk| {
            format!(
                "{}: {} / {} MB",
                disk.name().to_string_lossy(),
                disk.total_space() / 1024 / 1024 - disk.available_space() / 1024 / 1024,
                disk.total_space() / 1024 / 1024
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ");

    let uptime = sysinfo::System::uptime();
    let chrono_uptime = chrono::Duration::seconds(uptime as i64);

    let os_name = System::name();
    let kernel_version = System::kernel_version();
    let os_version = System::os_version();

    format!(
        "System Status:\nMem Usage: {}\nSwap Usage: {}\nCPU Usage: {}\nDisk Usage:\n  {}\nUptime: {} d {} h {} m\nOS: {} {} {}\n",
        mem_usage,
        swap_usage,
        cpu_usage,
        disk_usage,
        chrono_uptime.num_days(),
        chrono_uptime.num_hours() % 24,
        chrono_uptime.num_minutes() % 60,
        os_name.unwrap_or_else(|| "Unknown".to_string()),
        kernel_version.unwrap_or_else(|| "Unknown".to_string()),
        os_version.unwrap_or_else(|| "Unknown".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sys_status() {
        println!("{}", sys_status());
    }
}
