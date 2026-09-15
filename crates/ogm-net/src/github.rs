use async_trait::async_trait;

use crate::NetError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub published_at: Option<String>,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseOutcome {
    Released(Release),
    NotModified,
    NotFound,
}

#[async_trait]
pub trait ReleasesSource: Send + Sync {
    async fn latest_release(
        &self,
        repo: &str,
        etag: Option<&str>,
    ) -> Result<ReleaseOutcome, NetError>;
}

#[derive(Debug, Clone)]
pub struct GithubClient {
    client: reqwest::Client,
    token: Option<String>,
}

impl GithubClient {
    pub fn new(token: Option<String>) -> Result<Self, NetError> {
        let client = reqwest::Client::builder().user_agent("ogm").build()?;
        Ok(GithubClient { client, token })
    }
}

/// Parse the body of GET /repos/{owner}/{repo}/releases/latest.
pub fn parse_release_response(body: &str) -> Result<Release, NetError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| NetError::Parse(e.to_string()))?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .ok_or_else(|| NetError::Parse("missing tag_name".into()))?
        .to_string();
    let url = v
        .get("html_url")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let published_at = v
        .get("published_at")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    Ok(Release {
        tag,
        url,
        published_at,
        etag: None,
    })
}

#[async_trait]
impl ReleasesSource for GithubClient {
    async fn latest_release(
        &self,
        repo: &str,
        etag: Option<&str>,
    ) -> Result<ReleaseOutcome, NetError> {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let mut req = self.client.get(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        if let Some(etag) = etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let resp = req.send().await?;
        match resp.status() {
            reqwest::StatusCode::NOT_MODIFIED => Ok(ReleaseOutcome::NotModified),
            reqwest::StatusCode::NOT_FOUND => Ok(ReleaseOutcome::NotFound),
            reqwest::StatusCode::FORBIDDEN
                if resp
                    .headers()
                    .get("x-ratelimit-remaining")
                    .and_then(|v| v.to_str().ok())
                    == Some("0") =>
            {
                Err(NetError::RateLimited)
            }
            status if status.is_success() => {
                let etag = resp
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let body = resp.text().await?;
                let mut release = parse_release_response(&body)?;
                release.etag = etag;
                Ok(ReleaseOutcome::Released(release))
            }
            status => Err(NetError::Status(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_JSON: &str = r#"{
        "url": "https://api.github.com/repos/HarbourMasters/Shipwright/releases/261234567",
        "tag_name": "9.1.2",
        "name": "Ackbar Alfa",
        "draft": false,
        "prerelease": false,
        "created_at": "2026-09-01T00:00:00Z",
        "published_at": "2026-09-01T00:00:00Z",
        "html_url": "https://github.com/HarbourMasters/Shipwright/releases/tag/9.1.2",
        "assets": [{"name": "SoH-Ackbar-Alfa-Linux.zip"}]
    }"#;

    #[test]
    fn parses_github_release_fixture() {
        let r = parse_release_response(RELEASE_JSON).unwrap();
        assert_eq!(r.tag, "9.1.2");
        assert_eq!(
            r.url,
            "https://github.com/HarbourMasters/Shipwright/releases/tag/9.1.2"
        );
        assert_eq!(r.published_at.as_deref(), Some("2026-09-01T00:00:00Z"));
        assert!(r.etag.is_none());
    }

    #[test]
    fn missing_tag_is_parse_error() {
        assert!(parse_release_response("{}").is_err());
        assert!(parse_release_response("not json").is_err());
    }
}
