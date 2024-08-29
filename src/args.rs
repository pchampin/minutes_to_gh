use chrono::NaiveDate;
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// IRC channel from where the minutes are generated
    #[arg(short, long, env = "M2G_CHANNEL")]
    pub channel: String,

    /// Github token used to create comments
    #[arg(short, long, env = "M2G_TOKEN")]
    pub token: String,

    /// Date of the minutes, formatted as YYYY-MM-DD
    #[arg(short, long, env="M2G_DATE", default_value_t=today())]
    pub date: NaiveDate,

    /// Do not actually perform the operations on GitHub
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// File to read minutes from (rather than from the web)
    #[arg(short, long, env = "M2G_FILE")]
    pub file: Option<String>,

    /// Log-level (error, warn, info, debug, trace)
    #[arg(short, long, env = "M2G_LOG_LEVEL", default_value = "info")]
    pub log_level: log::LevelFilter,
}

fn today() -> NaiveDate {
    chrono::offset::Local::now().date_naive()
}
