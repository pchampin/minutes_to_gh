use chrono::NaiveDate;
use clap::{Args, Parser, Subcommand};

/// Comment github issues with links to meeting minutes
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CmdArgs {
    /// Github token used to create comments
    #[arg(short, long, env = "M2G_TOKEN")]
    pub token: String,

    /// Log-level (error, warn, info, debug, trace)
    #[arg(short, long, env = "M2G_LOG_LEVEL", default_value = "info")]
    pub log_level: log::LevelFilter,

    #[command(subcommand)]
    pub subcommand: SubCmdArgs,
}

/// Subcommands
#[derive(Subcommand, Clone, Debug)]
pub enum SubCmdArgs {
    /// Run an IRC bot that can comment github issues
    IrcBot(IrcBotArgs),
    /// Comment github issues from the command line
    Manual(EngineArgs),
}

/// See [`SubCmdArgs::Manual`]
#[derive(Args, Clone, Debug)]
pub struct EngineArgs {
    /// IRC channel from where the minutes were generated
    #[arg(short, long, env = "M2G_CHANNEL")]
    pub channel: String,

    /// Date of the minutes, formatted as YYYY-MM-DD
    #[arg(short, long, env = "M2G_DATE", default_value_t = today())]
    pub date: NaiveDate,

    /// Do not actually perform the operations on GitHub
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// URL to read minutes from (default: constructed from channel and date)
    #[arg(long, env = "M2G_URL")]
    pub url: Option<String>,

    /// File to read minutes from (default: fetched from URL)
    #[arg(long, env = "M2G_FILE")]
    pub file: Option<String>,
}

/// See [`SubCmdArgs::IrcBot`]
#[derive(Args, Clone, Debug)]
pub struct IrcBotArgs {
    /// Nickname used by the bot
    #[arg(short, long, default_value = "m2gbot", env = "M2G_NICKNAME")]
    pub nickname: String,

    /// IRC server
    #[arg(short, long, default_value = "irc.w3.org", env = "M2G_SERVER")]
    pub server: String,

    /// Port of the IRC server
    #[arg(short, long, default_value_t = 6679, env = "M2G_PORT")]
    pub port: u16,

    /// Username of the owner of the bot
    #[arg(short, long, env = "M2G_USERNAME")]
    pub username: Option<String>,

    /// Password of the owner of the bot (if needed)
    #[arg(short = 'P', long, env = "M2G_PASSWORD")]
    pub password: Option<String>,

    /// Channels on which the bot should connect automatically (comma separated)
    #[arg(short, long, env = "M2G_CHANNELS")]
    pub channels: Vec<String>,
}

impl From<IrcBotArgs> for irc::client::prelude::Config {
    fn from(value: IrcBotArgs) -> Self {
        Self {
            username: value.username,
            password: value.password,
            server: Some(value.server),
            port: Some(value.port),
            nickname: Some(value.nickname),
            encoding: Some("UTF-8".to_string()),
            realname: Some(
                "Minutes to Github bot: https://github.com/pchampin/minutes_to_gh".to_string(),
            ),
            channels: value.channels,
            ..Self::default()
        }
    }
}

fn today() -> NaiveDate {
    chrono::offset::Local::now().date_naive()
}
