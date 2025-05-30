//! I define types to handle repositories.json files on <https://www.github.com/w3c/groups>

use serde::Deserialize;

use crate::outcome::Issue;

#[derive(Clone, Debug, Deserialize)]
/// JSON structure describing a github repository
pub struct Repository {
    /// The name (identifier) of this repository
    pub name: String,
    /// The owner of this repository
    pub owner: Owner,
}

impl Repository {
    /// Determines whether a given issue is part of this repository
    pub fn contains(&self, issue: &Issue) -> bool {
        issue.owner == self.owner.login && issue.repo == self.name
    }
}

impl From<&str> for Repository {
    fn from(value: &str) -> Self {
        let parts = value.splitn(2, '/').collect::<Vec<_>>();
        let (org, repo) = match parts[..] {
            [repo] => ("w3c", repo),
            [org, repo] => (org, repo),
            _ => unreachable!(),
        };
        Self {
            name: repo.into(),
            owner: org.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
/// JSON structure describing the owner of a github repository
pub struct Owner {
    /// The github login of this owner
    pub login: String,
}

impl From<&str> for Owner {
    fn from(value: &str) -> Self {
        Owner {
            login: value.into(),
        }
    }
}
