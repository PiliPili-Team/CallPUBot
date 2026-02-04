use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "查看帮助")]
    Help,
    #[command(
        description = "或 c 一键被打"
    )]
    CallPU,
    #[command(description = "或 r 注册到被 Call 列表")]
    Register,
    #[command(description = "或 l 离开被 Call 列表")]
    Leave,
    #[command(description = "查看发送消息者被谁注册")]
    WhoRegisteredMe,
    #[command(description = "将自己加入 Call 黑名单")]
    Blacklist,
    #[command(description = "将自己从 Call 黑名单移除")]
    Unblacklist,
}
