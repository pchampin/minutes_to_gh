use anyhow::Result;
use futures::prelude::*;
use irc::client::prelude::*;
use regex::{Regex, RegexBuilder};

use std::sync::LazyLock;

use crate::{
    args::{EngineArgs, IrcBotArgs},
    engine::Engine,
    outcome::{
        Outcome,
        OutcomeKind::{Created, Faked, Skipped},
    },
};

pub async fn command(token: String, args: IrcBotArgs) -> Result<()> {
    Bot::new(token, args).await?.poll().await?;
    Ok(())
}

struct Bot {
    client: Client,
    token: String,
}

impl Bot {
    async fn new(token: String, args: IrcBotArgs) -> Result<Self> {
        log::info!("Connecting to {}:{}", args.server, args.port);
        let client = Client::from_config(args.into()).await?;
        // identify comes from ClientExt
        client.identify()?;
        log::info!("Identified as {}", client.current_nickname());
        Ok(Self { client, token })
    }

    async fn poll(&mut self) -> Result<()> {
        let mut stream = self.client.stream()?;
        while let Some(message) = stream.next().await.transpose()? {
            match &message.command {
                Command::INVITE(_, channel) => match self.client.send_join(channel) {
                    Ok(_) => log::info!("joining {channel} after being invited"),
                    Err(err) => log::error!("IRC error: {err:?}"),
                },
                Command::PRIVMSG(channel, content) => {
                    if let Some(cmd_str) = self.for_me(content) {
                        let cmd = BotCommand::from(cmd_str);
                        log::debug!("on {channel} got {cmd:?}, parsed from {cmd_str:?}");
                        let res = match cmd {
                            BotCommand::Bye => self.bye(channel),
                            BotCommand::Help => self.help(&message),
                            BotCommand::LinkIssues => self.link_issues(&message).await,
                            BotCommand::Debug => self.debug(&message).await,
                            BotCommand::Unrecognized => self.unrecognized(&message, cmd_str),
                        };
                        if let Err(err) = res {
                            log::error!("Error: {err:?}");
                            self.respond(&message, &format!("Something wrong happened: {err}"))
                                .unwrap_or(());
                        }
                    }
                }
                Command::KICK(chanlist, _, _) => {
                    log::info!("leaving {chanlist} after being kicked");
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn for_me<'a>(&self, message: &'a str) -> Option<&'a str> {
        let content = if message.starts_with("\u{1}ACTION ") {
            &message[8..message.len() - 1].trim()
        } else {
            &message.trim()
        };
        let nickname = self.client.current_nickname();
        if content.starts_with(nickname) && content[nickname.len()..].starts_with(", ") {
            Some(&content[nickname.len() + 2..])
        } else {
            None
        }
    }

    fn bye(&self, channel: &str) -> Result<()> {
        if channel.is_channel_name() {
            self.client.send_part(channel)?;
        }
        Ok(())
    }

    fn help(&self, message: &Message) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let nickname = message.source_nickname().unwrap_or("people");

        self.respond(
            message,
            &format!("{nickname}, I am {}.", env!("CARGO_PKG_DESCRIPTION"),),
        )?;
        self.respond(
            message,
            &format!(
                "... I am an instance of {} version {}.",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ),
        )?;
        self.respond(
            message,
            &format!("... To know more, see {}", env!("CARGO_PKG_HOMEPAGE")),
        )
    }

    async fn link_issues(&self, message: &Message) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));
        log::info!("Linking issues on {}", message.response_target().unwrap());

        self.do_link_issues(
            message,
            EngineArgs {
                channel: message.response_target().unwrap().to_string(),
                date: chrono::offset::Local::now().date_naive(),
                dry_run: false,
                url: None,
                file: None,
            },
        )
        .await
    }

    async fn debug(&self, message: &Message) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));
        log::info!("Debug on {}", message.response_target().unwrap());

        self.do_link_issues(
            message,
            EngineArgs {
                channel: message.response_target().unwrap().to_string(),
                // channel: "did".into(),
                date: chrono::offset::Local::now().date_naive(),
                // date: "2024-08-22".parse().unwrap(),
                dry_run: true,
                url: None,
                file: None,
            },
        )
        .await
    }

    async fn do_link_issues(&self, message: &Message, args: EngineArgs) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let engine = Engine::new(self.token.clone(), args).await?;
        engine
            .run()
            .try_for_each(|outcome: Outcome| async move {
                let issue = &outcome.issue;
                match outcome.kind {
                    Created(comment) => {
                        self.respond(message, &format!("comment created: {comment}"))
                    }
                    Faked => self.respond(
                        message,
                        &format!("comment would have been created for: {issue}"),
                    ),
                    Skipped(comment) => {
                        self.respond(message, &format!("comment already there: {comment}"))
                    }
                }
            })
            .await
    }

    fn unrecognized(&self, message: &Message, cmd_str: &str) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let nickname = message.source_nickname().unwrap_or("people");
        self.respond(
            message,
            &format!("sorry {nickname}, I don't understand {cmd_str:?}"),
        )
    }

    fn respond(&self, message: &Message, response: &str) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let Command::PRIVMSG(_, msg_str) = &message.command else {
            unreachable!();
        };
        let action = msg_str.starts_with("\u{1}ACTION ");
        let target = message.response_target().unwrap();
        if action {
            self.client.send_action(target, response)?;
        } else {
            self.client.send_privmsg(target, response)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum BotCommand {
    Bye,
    Help,
    LinkIssues,
    Debug,
    Unrecognized,
}

macro_rules! lazy_re {
    ($name:ident = $re:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| {
            RegexBuilder::new($re)
                .case_insensitive(true)
                .build()
                .unwrap()
        });
    };
}

impl From<&'_ str> for BotCommand {
    fn from(value: &str) -> Self {
        use BotCommand::*;

        lazy_re! { LINK_ISSUES = "^(please )?(back)?link (github )?issues( to minutes)?$" }
        lazy_re! { LINK_HELP = "^(please )?help$" }
        lazy_re! { LINK_BYE = "^bye|out|(please )?(excuse us|leave|part)$" }

        if LINK_ISSUES.is_match(value) {
            LinkIssues
        } else if LINK_HELP.is_match(value) {
            Help
        } else if LINK_BYE.is_match(value) {
            Bye
        } else if value == "debug" {
            Debug
        } else {
            Unrecognized
        }
    }
}
