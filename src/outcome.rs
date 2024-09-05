use crate::issue::Issue;

#[derive(Clone, Debug)]
pub struct Outcome {
    pub kind: OutcomeKind,
    pub issue: String,
}

#[derive(Clone, Debug)]
pub enum OutcomeKind {
    /// A comment was created for this issue (URL or the comment)
    Created(String),
    /// A comment was not created because of dry-run mode
    Faked,
    /// This issue was skipped because of a comment pointing to the minutes already exists (URL of the comment)
    Skipped(String),
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
    pub fn skipped(issue: Issue, comment: impl ToString) -> Self {
        Self {
            kind: OutcomeKind::Skipped(comment.to_string()),
            issue: issue.url.to_string(),
        }
    }
}
