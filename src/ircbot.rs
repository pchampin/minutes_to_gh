use anyhow::Result;
use futures::prelude::*;
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use irc::client::prelude::*;
use regex::{Regex, RegexBuilder};

use std::{sync::LazyLock, time::Duration};

use crate::{
    args::{EngineArgs, FinitePositiveF64, IrcBotArgs},
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
    governor: DefaultKeyedRateLimiter<String>,
}

impl Bot {
    async fn new(token: String, args: IrcBotArgs) -> Result<Self> {
        log::info!("Connecting to {}:{}", args.server, args.port);
        let client = Client::from_config(args.into()).await?;
        // identify comes from ClientExt
        client.identify()?;
        log::info!("Identified as {}", client.current_nickname());
        let governor =
            RateLimiter::keyed(Quota::with_period(Duration::from_secs_f64(1.0)).unwrap());
        Ok(Self {
            client,
            token,
            governor,
        })
    }

    async fn poll(&mut self) -> Result<()> {
        // the spawn below ensures that messages are sent as soon as client.send_X is called,
        // rather than on the next poll to the client.stream
        tokio::spawn(self.client.outgoing().unwrap());
        let mut stream = self.client.stream()?;
        while let Some(message) = stream.next().await.transpose()? {
            match &message.command {
                Command::INVITE(_, channel) => {
                    self.governor.until_key_ready(channel).await;
                    match self.client.send_join(channel) {
                        Ok(_) => log::info!("joining {channel} after being invited"),
                        Err(err) => log::error!("IRC error: {err:?}"),
                    }
                }
                Command::PRIVMSG(channel, content) => {
                    if let Some(cmd_str) = self.for_me(content) {
                        let cmd = BotCommand::from(cmd_str);
                        log::debug!("on {channel} got {cmd:?}, parsed from {cmd_str:?}");
                        let res = match cmd {
                            BotCommand::Bye => self.bye(channel).await,
                            BotCommand::Help => self.help(&message).await,
                            BotCommand::LinkIssues => self.link_issues(&message).await,
                            BotCommand::Debug => self.debug(&message).await,
                            BotCommand::Unrecognized => self.unrecognized(&message, cmd_str).await,
                        };
                        if let Err(err) = res {
                            log::error!("Error: {err:?}");
                            self.respond(&message, &format!("Something wrong happened: {err}"))
                                .await
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

    async fn bye(&self, channel: &String) -> Result<()> {
        self.governor.until_key_ready(channel).await;
        if channel.is_channel_name() {
            self.client.send_part(channel)?;
        }
        Ok(())
    }

    async fn help(&self, message: &Message) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let nickname = message.source_nickname().unwrap_or("people");

        self.respond(
            message,
            &format!("{nickname}, I am {}.", env!("CARGO_PKG_DESCRIPTION"),),
        )
        .await?;
        self.respond(
            message,
            &format!(
                "... I am an instance of {} version {}.",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ),
        )
        .await?;
        self.respond(
            message,
            &format!("... To know more, see {}", env!("CARGO_PKG_HOMEPAGE")),
        )
        .await
    }

    async fn link_issues(&self, message: &Message) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));
        log::info!("Linking issues on {}", message.response_target().unwrap());

        self.do_link_issues(
            message,
            EngineArgs {
                channel: message.response_target().unwrap().to_string(),
                date: chrono::offset::Local::now().date_naive(),
                rate_limit: FinitePositiveF64::new_unchecked(1.0),
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
                date: chrono::offset::Local::now().date_naive(),
                // channel: "did".into(),
                // date: "2024-08-22".parse().unwrap(),
                rate_limit: FinitePositiveF64::new_unchecked(1.0),
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
                            .await
                    }
                    Faked => {
                        self.respond(
                            message,
                            &format!("comment would have been created for: {issue}"),
                        )
                        .await
                    }
                    Skipped(comment) => {
                        self.respond(message, &format!("comment already there: {comment}"))
                            .await
                    }
                }
            })
            .await
    }

    async fn unrecognized(&self, message: &Message, cmd_str: &str) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let nickname = message.source_nickname().unwrap_or("people");
        self.respond(
            message,
            &format!("sorry {nickname}, I don't understand {cmd_str:?}"),
        )
        .await
    }

    async fn respond(&self, message: &Message, response: &str) -> Result<()> {
        debug_assert!(matches!(message.command, Command::PRIVMSG(..)));

        let Command::PRIVMSG(target, msg_str) = &message.command else {
            unreachable!();
        };
        let action = msg_str.starts_with("\u{1}ACTION ");
        let target = my_response_target(target, message).unwrap();
        self.governor.until_key_ready(target).await;
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

/// Version of Message:response_target that returns &Strings instead of &str,
/// so that we can pass it as keys to Bot::governor
fn my_response_target<'a>(target: &'a String, msg: &'a Message) -> Option<&'a String> {
    if target.is_channel_name() {
        Some(target)
    } else if let Prefix::Nickname(name, _, _) = msg.prefix.as_ref()? {
        Some(name)
    } else {
        None
    }
}
