use anyhow::Result;

use nekoton::external;

pub struct GqlConnection {
    url: reqwest::Url,
    client: reqwest::Client,
}

impl GqlConnection {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: url.parse::<reqwest::Url>()?,
            client: Default::default(),
        })
    }
}

#[async_trait::async_trait]
impl external::GqlConnection for GqlConnection {
    async fn post(&self, data: &str) -> Result<String> {
        let data = data.to_string();
        let req = self
            .client
            .post(self.url.clone())
            .body(data)
            .header("Content-Type", "application/json")
            .build()?;

        let result = self.client.execute(req).await?.text().await?;
        Ok(result)
    }
}
