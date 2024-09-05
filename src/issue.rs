use std::sync::OnceLock;

use regex::Regex;

#[derive(Clone, Copy, Debug)]
pub struct Issue<'a> {
    pub url: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
    pub id: u64,
}

impl<'a> Issue<'a> {
    pub fn try_from_url(url: &'a str) -> Option<Self> {
        static RE_ISSUE: OnceLock<Regex> = OnceLock::new();
        let groups = RE_ISSUE
            .get_or_init(|| {
                Regex::new(r"//github.com/([^/]+)/([^/]+)/(issues|pull|#)/([0-9]+)$").unwrap()
            })
            .captures(url)?;
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
