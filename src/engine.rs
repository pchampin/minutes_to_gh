use std::sync::LazyLock;
use std::{iter::once, sync::OnceLock};

use anyhow::{Context, Result};
use async_stream::try_stream;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use ego_tree::NodeRef;
use futures::Stream;
use html2md_rs::to_md::safe_from_html_to_md;
use octocrab::{issues::IssueHandler, models::issues::Comment, Octocrab};
use regex::Regex;
use reqwest::Response;
use scraper::Node;
use scraper::{
    ElementRef, Html,
    Node::{Element, Text},
    Selector,
};

use crate::args::EngineArgs;
use crate::doc_fragment::DocFragment;
use crate::issue::Issue;
use crate::outcome::Outcome;

/// The engine of this create, locating mentions to GitHub issues/PRs in minutes,
/// and commenting the corresponding issue/PR with a link to the relevant part of the minutes.
pub struct Engine {
    url: String,
    dom: Html,
    github: Octocrab,
    min_date: DateTime<Utc>,
    message_template: String,
    dry_run: bool,
}

impl Engine {
    pub async fn new(token: String, args: EngineArgs) -> Result<Self> {
        let url = args.url.unwrap_or_else(|| {
            format!(
                "https://www.w3.org/{}/{:02}/{:02}-{}-minutes.html",
                args.date.year(),
                args.date.month(),
                args.date.day(),
                args.channel,
            )
        });
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

        let github = Octocrab::builder().personal_token(token).build()?;
        let min_date = NaiveDateTime::from(args.date.pred_opt().unwrap()).and_utc();
        let message_template = format!(
            "This was discussed during the [{} meeting on {}](%URL%).\n\n<details><summary><i>View the transcript</i></summary>\n\n%FRAGMENT%\n----\n</details>",
            args.channel,
            args.date.format("%d %B %Y"),
        );

        Ok(Self {
            url,
            dom,
            github,
            min_date,
            message_template,
            dry_run: args.dry_run,
        })
    }

    // Run the engine and yield a number of outcomes.
    pub fn run(&self) -> impl Stream<Item = Result<Outcome>> + '_ {
        try_stream! {
            for (issue, link, fragment) in issues_with_link(&self.dom, &self.url) {
                log::debug!("{} referenced in {link}", issue.url);
                let issues = self.github.issues(issue.owner, issue.repo);
                if let Some(comment) = comment_to_link(&link, &issues, issue.id, self.min_date).await? {
                    log::info!(
                        "Skipping {issue}, link to minutes already there: {}",
                        comment.html_url,
                    );
                    yield Outcome::skipped(issue, comment.html_url);
                    continue;
                }
                let message = self.message_template
                    .replace("%URL%", &link)
                    .replace("%FRAGMENT%", &fragment);
                log::trace!("Comment message: {message}");
                if self.dry_run {
                    log::info!("Comment posted: (not really, running in dry mode)");
                    yield Outcome::faked(issue);
                    continue;
                }
                let comment = issues
                    .create_comment(issue.id, message)
                    .await?;
                log::info!("Comment posted: {}", comment.html_url);
                yield Outcome::created(issue, comment.html_url);
            }
        }
    }
}

/// Iter over all github issues cited in dom,
/// together with the most appropriate link to refer to where this issue was discussed.
fn issues_with_link<'a>(
    dom: &'a Html,
    url: &'a str,
) -> impl Iterator<Item = (Issue<'a>, String, String)> {
    static SEL: OnceLock<Selector> = OnceLock::new();
    let sel = SEL.get_or_init(|| Selector::parse("a").unwrap());
    dom.select(sel)
        .map(|a| (a, a.attr("href").and_then(Issue::try_from_url)))
        .filter_map(transpose_2nd)
        .map(|(a, issue)| (issue, find_closest_hn_id(a)))
        .filter_map(transpose_2nd)
        .map(move |(issue, fragment)| {
            (issue, format!("{}#{}", &url, fragment.id), fragment.content)
        })
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
fn find_closest_hn_id(e: ElementRef) -> Option<DocFragment> {
    element_ancestors(e)
        .flat_map(element_prev_siblings)
        .filter_map(try_as_fragment_boundary)
        .map(|(id, e)| extract_fragment(id, e))
        .next()
}

/// If this element is a fragment boundary (i.e. a hn with and id),
/// return its id and itself, otherwise, return None.
fn try_as_fragment_boundary(e: ElementRef) -> Option<(&str, ElementRef)> {
    static RE_HN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[hH][1234]$").unwrap());
    let v = e.value();
    if !RE_HN.is_match(v.name()) {
        None
    } else {
        v.attr("id").map(|id| (id, e))
    }
}

/// Extract fragment reachable from this element if it has an id.
fn extract_fragment<'a>(id: &'a str, e: ElementRef<'a>) -> DocFragment<'a> {
    debug_assert!(e.value().attr("id") == Some(id));
    let e_frag = e.html();
    let s_frags = e
        .next_siblings()
        .take_while(not_fragment_boundary)
        .filter_map(|n| match n.value() {
            Text(txt) => Some(txt.to_string()),
            Element(_) => ElementRef::wrap(n).map(|er| er.html()),
            _ => None,
        });
    let html = once(e_frag).chain(s_frags).collect::<Vec<_>>().join("");
    let content = safe_from_html_to_md(html).unwrap_or("".into());
    DocFragment { id, content }
}

fn not_fragment_boundary(n: &NodeRef<Node>) -> bool {
    let Some(e) = ElementRef::wrap(*n) else {
        return true;
    };
    try_as_fragment_boundary(e).is_none()
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
