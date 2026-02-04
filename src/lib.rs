use bot::Bot;
use tracing_subscriber::fmt::time::ChronoLocal;

mod bot;
mod call_map;
mod cmd;
mod question;
mod msg_prelude;

pub use call_map::*;
pub use msg_prelude::*;

pub const BOT_TOKEN: &str = "";
pub const WHITE_GROUP: i64 = 0;

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_timer(ChronoLocal::rfc_3339()).with_max_level(tracing::Level::INFO).init();

    let bot: Bot = bot::Bot::new();
    bot.run_active().await
}
