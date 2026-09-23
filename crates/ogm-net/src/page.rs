use async_trait::async_trait;

use crate::NetError;

const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: String,
}

#[async_trait]
pub trait PageSource: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Page, NetError>;
}

#[derive(Debug, Clone)]
pub struct PageClient {
    client: reqwest::Client,
}

impl PageClient {
    pub fn new() -> Result<Self, NetError> {
        // Browser-shaped UA: several fan-project hosts (rallysimfans et al.)
        // 403 anything that doesn't look like a browser. The ogm suffix keeps
        // us identifiable in their logs.
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 ",
                "(KHTML, like Gecko) Chrome/126.0 Safari/537.36 ogm/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(PageClient { client })
    }
}

#[async_trait]
impl PageSource for PageClient {
    async fn fetch(&self, url: &str) -> Result<Page, NetError> {
        let mut resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Status(resp.status()));
        }
        let etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let last_modified = resp
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let mut body = Vec::new();
        while let Some(chunk) = resp.chunk().await? {
            if body.len() + chunk.len() > MAX_BODY {
                body.extend_from_slice(&chunk[..MAX_BODY - body.len()]);
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Page {
            etag,
            last_modified,
            body: String::from_utf8_lossy(&body).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_client_builds() {
        assert!(PageClient::new().is_ok());
    }
}
