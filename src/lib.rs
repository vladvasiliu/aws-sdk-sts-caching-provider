use aws_types::credentials::future::ProvideCredentials;
use aws_types::credentials::CredentialsError;
use aws_types::{credentials, Credentials};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::debug;

/// A caching CredentialsProvider that retrieves credentials from STS.
///
/// STS is queried using environment credentials.
///
/// The credentials are retrieved via an `AssumeRole` call. The fields of the struct reflect the
/// parameters for that call.
///
/// The `cache_timeout` field represents how many seconds in the future the temporary token must be
/// valid before being considered stale.
#[derive(Debug)]
pub struct STSCredentialsProvider {
    role_arn: String,
    external_id: Option<String>,
    source_identity: Option<String>,
    cred_cache: RwLock<Option<Credentials>>,
    session_name: Option<String>,
    session_duration: Option<i32>,
    cache_timeout: u64,
}

impl STSCredentialsProvider {
    pub fn new(
        role_arn: &str,
        external_id: Option<&str>,
        source_identity: Option<&str>,
        session_name: Option<&str>,
        session_duration: Option<i32>,
        cache_timeout: u64,
    ) -> Self {
        Self {
            role_arn: role_arn.to_string(),
            external_id: external_id.map(String::from),
            source_identity: source_identity.map(String::from),
            cred_cache: RwLock::new(None),
            session_name: session_name.map(String::from),
            session_duration,
            cache_timeout,
        }
    }

    /// Returns the stored credentials if they are valid
    /// The credentials are valid iff
    /// * they're not None
    /// * they expire at least `cache_timeout` seconds in the future
    async fn stored_credentials(&self) -> Option<Credentials> {
        let lock = self.cred_cache.read().await;
        if let Some(c) = lock.as_ref() {
            let min_expiry_time = SystemTime::now() + Duration::from_secs(self.cache_timeout);
            let expiration = c.expiry().unwrap();
            if expiration > min_expiry_time {
                return Some(c.clone());
            }
        }
        None
    }

    async fn load_credentials(&self) -> aws_types::credentials::Result {
        let sts_config = aws_config::load_from_env().await;
        let sts_client = aws_sdk_sts::client::Client::new(&sts_config);
        sts_client
            .assume_role()
            .role_arn(&self.role_arn)
            .set_role_session_name(self.session_name.clone())
            .set_external_id(self.external_id.clone())
            .set_source_identity(self.source_identity.clone())
            .set_duration_seconds(self.session_duration)
            .send()
            .await
            .map_err(CredentialsError::provider_error)?
            .credentials
            .map(|c| {
                credentials::Credentials::new(
                    c.access_key_id.unwrap(),
                    c.secret_access_key.unwrap(),
                    c.session_token,
                    c.expiration.map(|e| e.try_into().unwrap()),
                    "STSCredentialsProvider",
                )
            })
            .ok_or_else(|| {
                CredentialsError::not_loaded("STS Assume Role returned no credentials".to_string())
            })
    }

    /// Returns the credentials from cache or updates the cache if they're expired
    async fn get_credentials(&self) -> aws_types::credentials::Result {
        match self.stored_credentials().await {
            Some(creds) => {
                debug!("Returning cached credentials");
                Ok(creds)
            }
            None => {
                debug!("No valid credentials in cache. Getting from STS");
                let mut lock = self.cred_cache.write().await;
                let new_creds = self.load_credentials().await;
                match &new_creds {
                    Ok(creds) => *lock = Some(creds.clone()),
                    Err(_) => *lock = None,
                };
                new_creds
            }
        }
    }
}

impl credentials::ProvideCredentials for STSCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> ProvideCredentials<'a>
    where
        Self: 'a,
    {
        aws_types::credentials::future::ProvideCredentials::new(self.get_credentials())
    }
}
