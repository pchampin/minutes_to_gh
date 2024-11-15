use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug)]
pub struct Outcome {
    pub kind: OutcomeKind,
    pub issue: String,
}

#[derive(Debug)]
pub enum OutcomeKind {
    /// A comment was created for this issue (URL or the comment)
    Created(String),
    /// A comment was not created because of dry-run mode
    Faked,
    /// This issue was skipped because of a comment pointing to the minutes already exists (URL of the comment)
    Duplicate(String),
    /// This issue was skipped because it is not in a repository owned by the current group
    NotOwned,
    /// An error occurred
    #[expect(dead_code)]
    Error(anyhow::Error),
}

impl Outcome {
    pub fn created(issue: Issue, comment: impl ToString) -> Self {
        Self {
            kind: OutcomeKind::Created(comment.to_string()),
            issue: issue.url.to_string(),
        }
    }
    pub fn faked(issue: Issue) -> Self {
        Self {
            kind: OutcomeKind::Faked,
            issue: issue.url.to_string(),
        }
    }
    pub fn duplicate(issue: Issue, comment: impl ToString) -> Self {
        Self {
            kind: OutcomeKind::Duplicate(comment.to_string()),
            issue: issue.url.to_string(),
        }
    }
    pub fn not_owned(issue: Issue) -> Self {
        Self {
            kind: OutcomeKind::NotOwned,
            issue: issue.url.to_string(),
        }
    }
    pub fn error(issue: Issue, error: anyhow::Error) -> Self {
        Self {
            kind: OutcomeKind::Error(error),
            issue: issue.url.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Issue<'a> {
    pub url: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
    pub id: u64,
}

impl<'a> Issue<'a> {
    pub fn try_from_url(url: &'a str) -> Option<Self> {
        static RE_ISSUE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"//github.com/([^/]+)/([^/]+)/(issues|pull|#)/([0-9]+)$").unwrap()
        });
        let groups = RE_ISSUE.captures(url)?;
        Some(Issue {
            url,
            owner: groups.get(1).unwrap().as_str(),
            repo: groups.get(2).unwrap().as_str(),
            id: groups.get(4).unwrap().as_str().parse().unwrap(),
        })
    }
}

impl<'a> std::fmt::Display for Issue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}#{}", self.owner, self.repo, self.id)
    }
}
