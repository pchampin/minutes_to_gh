use anyhow::Result;
use clap::Parser;

mod args;
mod engine;
mod ircbot;
mod issue;
mod manual;
mod outcome;

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::CmdArgs::parse();
    let token = args.token;

    env_logger::builder()
        .format_timestamp(None)
        .filter_module("minutes_to_gh", args.log_level)
        .init();

    match args.subcommand {
        args::SubCmdArgs::Manual(args) => manual::command(token, args).await,
        args::SubCmdArgs::IrcBot(args) => ircbot::command(token, args).await,
    }
}
