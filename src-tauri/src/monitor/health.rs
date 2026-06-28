use std::time::Duration;

pub struct HealthChecker {
    url: String,
    client: reqwest::blocking::Client,
}

impl HealthChecker {
    pub fn new(server_url: &str) -> Self {
        let url = format!("{}/session/status", server_url.trim_end_matches('/'));
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { url, client }
    }

    pub fn check_once(&self) -> bool {
        match self.client.get(&self.url).send() {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
