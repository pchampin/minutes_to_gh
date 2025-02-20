use std::str::FromStr;

use anyhow::{Error, Result};
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
    #[arg(
        short,
        long,
        env = "M2G_LOG_LEVEL",
        default_value = "info",
        global = true,
        display_order = 99
    )]
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

    /// Include transcript in GitHub comment
    #[arg(short = 'T', long, env = "M2G_TRANSCRIPT", default_value_t = false)]
    pub transcript: bool,

    /// Comma-separated list of groups concerned by these minutes (defaults to "wg/{channel}")
    #[arg(short, long, env = "M2G_GROUP")]
    pub groups: Option<String>,

    /// Minimum delay (in sec) between processing two issues (throttling GitHub API calls)
    #[arg(short, long, env = "M2G_RATE_LIMIT", default_value_t = FinitePositiveF64(0.2), value_parser = FinitePositiveF64::from_str, help_heading = "Advanced options", hide_short_help = true)]
    pub rate_limit: FinitePositiveF64,

    /// Do not actually perform the operations on GitHub
    #[arg(
        short = 'n',
        long,
        help_heading = "Advanced options",
        hide_short_help = true
    )]
    pub dry_run: bool,

    /// URL to read minutes from (default: constructed from channel and date)
    #[arg(
        long,
        env = "M2G_URL",
        help_heading = "Advanced options",
        hide_short_help = true
    )]
    pub url: Option<String>,

    /// File to read minutes from (default: fetched from URL)
    #[arg(
        long,
        env = "M2G_FILE",
        help_heading = "Advanced options",
        hide_short_help = true
    )]
    pub file: Option<String>,

    /// Allowed repository (in addition to those belonging to the group)
    ///
    /// The format of this argument is either '{org}/{repo}' or '{repo}'.
    /// In the latter case, the organization is assumed to be `w3c`.
    #[arg(
        long = "repository",
        help_heading = "Advanced options",
        hide_short_help = true
    )]
    pub extra_repositories: Vec<String>,
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

#[derive(Clone, Copy, Debug)]
pub struct FinitePositiveF64(f64);

impl FinitePositiveF64 {
    pub fn new_unchecked(value: f64) -> Self {
        debug_assert!(Self::try_from(value).is_ok());
        Self(value)
    }
}

impl TryFrom<f64> for FinitePositiveF64 {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(Error::msg(format!("{value} is not finite and positive")))
        }
    }
}

impl FromStr for FinitePositiveF64 {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        s.parse::<f64>()?.try_into()
    }
}

impl std::fmt::Display for FinitePositiveF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<FinitePositiveF64> for f64 {
    fn from(wrapper: FinitePositiveF64) -> Self {
        wrapper.0
    }
}
