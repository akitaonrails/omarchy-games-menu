pub mod github;
pub mod sgdb;

pub use github::{GithubClient, Release, ReleaseOutcome, ReleasesSource};
pub use sgdb::{ArtworkSource, SgdbClient, SgdbGame};

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("GitHub rate limit exceeded")]
    RateLimited,
    #[error("unexpected status {0}")]
    Status(reqwest::StatusCode),
    #[error("failed to parse response: {0}")]
    Parse(String),
    #[error("missing SteamGridDB API key")]
    MissingApiKey,
}
