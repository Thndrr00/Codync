use super::Connector;
use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
impl Connector {
    /// Token refresh and presentation changes do not replace a running server.
    pub(crate) fn runtime_identity(&self) -> Value {
        json!({"command":self.command,"args":self.args,"env":self.env,"url":self.url,"headers":self.headers,
            "oauth":self.oauth.as_ref().map(|oauth|json!({"clientId":oauth.client_id,"clientSecret":oauth.client_secret,
                "tokenEndpoint":oauth.token_endpoint,"redirectUris":oauth.redirect_uris,"signedIn":oauth.signed_in()}))})
    }
    pub(crate) fn runtime_fingerprint(&self) -> Result<String> {
        let bytes = Sha256::digest(serde_json::to_vec(&self.runtime_identity())?);
        Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_identity_includes_environment_but_excludes_cosmetic_and_rotating_tokens() {
        let original: Connector =
            serde_json::from_value(json!({"id":"one","name":"Old","url":"https://example.com/mcp",
            "oauth":{"clientId":"client","accessToken":"old","tokenEndpoint":"https://example.com/token"}}))
            .unwrap();
        let mut next = original.clone();
        next.name = "New".into();
        next.oauth.as_mut().unwrap().access_token = "refreshed".into();
        assert_eq!(original.runtime_identity(), next.runtime_identity());
        next.env.insert("MODE".into(), "changed".into());
        assert_ne!(original.runtime_identity(), next.runtime_identity());
    }
}
