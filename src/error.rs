use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineCreationError {
    #[error("Minutes not found <{0}>")]
    MinutesNotFound(String, #[source] reqwest::Error),
    #[error("Error loading minutes")]
    MinutesHttp(#[source] reqwest::Error),
    #[error("Failed loading minutes from file")]
    MinutesFile(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("W3C API error")]
    W3cApi(#[source] reqwest::Error),
    #[error("GitHub API error")]
    GitHub(#[from] octocrab::Error),
}

impl EngineCreationError {
    pub fn minutes(err: reqwest::Error) -> Self {
        if err.status().map(|s| s.as_u16()) == Some(404) {
            Self::MinutesNotFound(err.url().unwrap().to_string(), err)
        } else {
            Self::MinutesHttp(err)
        }
    }

    pub fn w3c_api(err: reqwest::Error) -> Self {
        Self::W3cApi(err)
    }
}
