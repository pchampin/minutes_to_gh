use std::{iter::once, sync::OnceLock};

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use clap::Parser;
use octocrab::{issues::IssueHandler, models::issues::Comment, Octocrab};
use regex::Regex;
use reqwest::Response;
use scraper::{ElementRef, Html, Selector};

mod args;
mod issue;

use issue::Issue;

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();

    env_logger::builder()
        .format_timestamp(None)
        .filter_module("minutes_to_gh", args.log_level)
        .init();

    let url = format!(
        "https://www.w3.org/{}/{:02}/{:02}-{}-minutes.html",
        args.date.year(),
        args.date.month(),
        args.date.day(),
        args.channel,
    );
    log::debug!("Minutes URL: {url:?}");

    let html = if let Some(filename) = args.file {
        log::debug!("Reading from file {filename} instead of URL");
        std::fs::read_to_string(&filename)
            .with_context(|| format!("Failed loading minutes from file {filename}"))?
    } else {
        reqwest::get(&url)
            .await
            .and_then(Response::error_for_status)
            .with_context(|| format!("Failed loading minutes from {url}"))?
            .text()
            .await?
    };
    let dom = Html::parse_document(&html);

    let github = Octocrab::builder().personal_token(args.token).build()?;
    let min_date = NaiveDateTime::from(args.date.pred_opt().unwrap()).and_utc();
    let message = format!(
        "This was discussed during the {} meeting on {:04}-{:02}-{:02}:",
        args.channel,
        args.date.year(),
        args.date.month(),
        args.date.day(),
    );

    for (issue, link) in issues_with_link(&dom, &url) {
        log::debug!("{} referenced in {link}", issue.url);
        let issues = github.issues(issue.owner, issue.repo);
        if let Some(comment) = comment_to_link(&link, &issues, issue.id, min_date).await? {
            log::warn!(
                "Skipping {issue}, link to minutes already there: {}",
                comment.html_url,
            );
            continue;
        }
        if args.dry_run {
            log::info!("Comment posted: (not really, running in dry mode)");
            continue;
        }
        let comment = issues
            .create_comment(issue.id, format!("{message}\n{link}"))
            .await?;
        log::info!("Comment posted: {}", comment.html_url);
    }

    Ok(())
}

/// Iter over all github issues cited in dom,
/// together with the most appropriate link to refer to where this issue was discussed.
fn issues_with_link<'a>(dom: &'a Html, url: &'a str) -> impl Iterator<Item = (Issue<'a>, String)> {
    static SEL: OnceLock<Selector> = OnceLock::new();
    let sel = SEL.get_or_init(|| Selector::parse("a").unwrap());
    dom.select(sel)
        .map(|a| (a, a.attr("href").and_then(Issue::try_from_url)))
        .filter_map(transpose_2nd)
        .map(|(a, href)| (href, find_closest_hn_id(a)))
        .filter_map(transpose_2nd)
        .map(move |(issue, id)| (issue, format!("{}#{}", &url, id)))
}

/// Find a comment citing `url` in the given issue, if any.
///
/// NB: only issue posted after `min_date` are explored,
/// and it is assumed that `min_date` is recent enough that no more than 200 comments are posted.
async fn comment_to_link(
    url: &str,
    issues: &IssueHandler<'_>,
    id: u64,
    min_date: DateTime<Utc>,
) -> Result<Option<Comment>> {
    Ok(issues
        .list_comments(id)
        .since(min_date)
        .per_page(200)
        .send()
        .await?
        .items
        .into_iter()
        .find(|comment| {
            comment
                .body
                .as_ref()
                .filter(|txt| txt.contains(url))
                .is_some()
        }))
}

/// Transpose a tuple on its 2nd component.
fn transpose_2nd<T, U>(pair: (T, Option<U>)) -> Option<(T, U)> {
    match pair {
        (_, None) => None,
        (a, Some(b)) => Some((a, b)),
    }
}

/// Find the header (h1, h2...) with an id which is closest before the given element.
fn find_closest_hn_id(e: ElementRef) -> Option<&str> {
    static RE_HN: OnceLock<Regex> = OnceLock::new();
    let re_hn = RE_HN.get_or_init(|| Regex::new(r"^[hH][1234]$").unwrap());
    element_ancestors(e)
        .flat_map(element_prev_siblings)
        .filter(|e| re_hn.is_match(e.value().name()))
        .filter_map(|e| e.value().attr("id"))
        .next()
}

/// Iterate over all ancestors of e that are elements.
///
/// "ancestors" here is to be understood in the broad sense:
/// e is yielded as its first ancestor.
fn element_ancestors(e: ElementRef) -> impl Iterator<Item = ElementRef> {
    once(e).chain(e.ancestors().filter_map(ElementRef::wrap))
}

/// Iterate over all previous siblings of e that are elements.
///
/// "sibling" here is to be understood in the broad sense:
/// e is yielded as its first sibling.
fn element_prev_siblings(e: ElementRef) -> impl Iterator<Item = ElementRef> {
    once(e).chain(e.prev_siblings().filter_map(ElementRef::wrap))
}
