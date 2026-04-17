use phoenix_types::PhoenixHttpError;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;

pub struct InviteClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl InviteClient<'_> {
    pub async fn activate_invite(
        &self,
        authority: &Pubkey,
        code: &str,
    ) -> Result<String, PhoenixHttpError> {
        let url = format!("{}/v1/invite/activate", self.http.api_url);

        let response = self
            .http
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "authority": authority.to_string(),
                "code": code,
            }))
            .send()
            .await
            .map_err(PhoenixHttpError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response
            .text()
            .await
            .map_err(|e| PhoenixHttpError::ParseFailed(format!("Failed to read response: {}", e)))
    }

    pub async fn activate_referral(
        &self,
        authority: &Pubkey,
        referral_code: &str,
    ) -> Result<String, PhoenixHttpError> {
        let url = format!("{}/v1/invite/activate-with-referral", self.http.api_url);

        let response = self
            .http
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "authority": authority.to_string(),
                "referral_code": referral_code,
            }))
            .send()
            .await
            .map_err(PhoenixHttpError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response
            .text()
            .await
            .map_err(|e| PhoenixHttpError::ParseFailed(format!("Failed to read response: {}", e)))
    }
}
