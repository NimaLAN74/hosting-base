//! IBKR Web API OAuth 1.0a first-party: Live Session Token (LST) for api.ibkr.com.
//!
//! Flow: load consumer key, access token/secret, and PEMs from config → compute LST (DH + HMAC-SHA1)
//! → cache LST until expiry (~24h) → use LST to sign subsequent API requests (HMAC-SHA256).
//! Keycloak remains the app IdP; this module provides server-side IBKR session for market data.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use num_bigint::BigInt;
use num_traits::Num;
use rand::Rng;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use rsa::signature::{Signature, Signer};
use rsa::traits::{Decryptor, PrivateKeyParts};
use rsa::RsaPrivateKey;
use serde::Deserialize;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::config::AppConfig;

type HmacSha1 = Hmac<Sha1>;

/// Cached Live Session Token and expiry (ms since epoch).
#[derive(Clone)]
struct CachedLst {
    token: String,
    expires_at_ms: u64,
}

/// IBKR OAuth 1.0a first-party client: obtains and caches Live Session Token.
pub struct IbkrOAuthClient {
    config: Arc<AppConfig>,
    /// Cached LST; refreshed when expired or missing.
    cache: RwLock<Option<CachedLst>>,
    /// Preloaded keys (avoid reading files on every LST refresh).
    keys: Option<IbkrKeys>,
}

struct IbkrKeys {
    dh_prime: BigInt,
    dh_generator: BigInt,
    encryption_key: RsaPrivateKey,
    signature_key: RsaPrivateKey,
}

fn percent_encode_rfc3986(s: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

impl IbkrOAuthClient {
    pub fn new(config: Arc<AppConfig>) -> Self {
        let keys = Self::load_keys(config.as_ref()).ok();
        if keys.is_none() && config.ibkr_oauth_configured() {
            tracing::warn!("IBKR OAuth env set but key files could not be loaded; IBKR API calls will fail until PEM paths are correct");
        }
        Self {
            config,
            cache: RwLock::new(None),
            keys,
        }
    }

    fn load_keys(config: &AppConfig) -> Result<IbkrKeys> {
        let dh_path = config
            .ibkr_oauth_dh_param_path
            .as_ref()
            .context("IBKR_OAUTH_DH_PARAM_PATH")?;
        let enc_path = config
            .ibkr_oauth_private_encryption_key_path
            .as_ref()
            .context("IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH")?;
        let sig_path = config
            .ibkr_oauth_private_signature_key_path
            .as_ref()
            .context("IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH")?;

        let dh_pem = std::fs::read_to_string(dh_path).context("read dh param PEM")?;
        let enc_pem = std::fs::read_to_string(enc_path).context("read encryption key PEM")?;
        let sig_pem = std::fs::read_to_string(sig_path).context("read signature key PEM")?;

        let dh_key = RsaPrivateKey::from_pkcs8_pem(&dh_pem).context("parse DH param as RSA key")?;
        let n_bytes = dh_key.n().to_bytes_be();
        let e_bytes = dh_key.e().to_bytes_be();
        let dh_prime = BigInt::from_bytes_be(num_bigint::Sign::Plus, &n_bytes);
        let dh_generator = BigInt::from_bytes_be(num_bigint::Sign::Plus, &e_bytes);

        let encryption_key = RsaPrivateKey::from_pkcs8_pem(&enc_pem).context("parse encryption key")?;
        let signature_key = RsaPrivateKey::from_pkcs8_pem(&sig_pem).context("parse signature key")?;

        Ok(IbkrKeys {
            dh_prime,
            dh_generator,
            encryption_key,
            signature_key,
        })
    }

    /// Returns true if IBKR OAuth is configured and key files loaded.
    pub fn is_configured(&self) -> bool {
        self.config.ibkr_oauth_configured() && self.keys.is_some()
    }

    /// Get a valid Live Session Token; refreshes if expired or missing. Uses blocking crypto on a spawn_blocking if needed.
    pub async fn get_live_session_token(&self) -> Result<String> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        {
            let guard = self.cache.read().await;
            if let Some(ref c) = *guard {
                if c.expires_at_ms > now_ms && (c.expires_at_ms - now_ms) > 60_000 {
                    return Ok(c.token.clone());
                }
            }
        }

        let config = self.config.clone();
        let keys = self.keys.clone().context("IBKR keys not loaded")?;
        let consumer_key = config
            .ibkr_oauth_consumer_key
            .as_ref()
            .context("missing consumer key")?
            .clone();
        let access_token = config
            .ibkr_oauth_access_token
            .as_ref()
            .context("missing access token")?
            .clone();
        let access_token_secret_b64 = config
            .ibkr_oauth_access_token_secret
            .as_ref()
            .context("missing access token secret")?
            .clone();
        let realm = config
            .ibkr_oauth_realm
            .as_deref()
            .unwrap_or("limited_poa")
            .to_string();
        let base_url = config.ibkr_api_base_url.clone();

        let (token, expires_at_ms) = tokio::task::spawn_blocking(move || {
            compute_live_session_token(
                &keys,
                &consumer_key,
                &access_token,
                &access_token_secret_b64,
                &realm,
                &base_url,
            )
        })
        .await
        .context("LST task join")??;

        {
            let mut guard = self.cache.write().await;
            *guard = Some(CachedLst {
                token: token.clone(),
                expires_at_ms,
            });
        }
        Ok(token)
    }

    /// Build Authorization header value for an IBKR API request (HMAC-SHA256 signed with LST).
    /// Method and url should be the actual HTTP method and full URL; optional body for POST.
    pub async fn sign_request(&self, method: &str, url: &str, _body: Option<&[u8]>) -> Result<String> {
        let lst = self.get_live_session_token().await?;
        let consumer_key = self.config.ibkr_oauth_consumer_key.as_deref().context("no consumer key")?;
        let access_token = self.config.ibkr_oauth_access_token.as_deref().context("no access token")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("oauth_consumer_key", consumer_key.to_string());
        params.insert("oauth_nonce", nonce);
        params.insert("oauth_signature_method", "HMAC-SHA256".to_string());
        params.insert("oauth_timestamp", timestamp.to_string());
        params.insert("oauth_token", access_token.to_string());

        let params_str = params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode_rfc3986(k), percent_encode_rfc3986(v)))
            .collect::<Vec<_>>()
            .join("&");
        let base_string = format!(
            "{}&{}&{}",
            method,
            percent_encode_rfc3986(url),
            percent_encode_rfc3986(&params_str)
        );

        let lst_bytes = BASE64.decode(&lst).context("decode LST")?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&lst_bytes).context("HMAC key")?;
        mac.update(base_string.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let oauth_signature = percent_encode_rfc3986(&BASE64.encode(sig_bytes));

        params.insert("oauth_signature", oauth_signature);
        let realm = self
            .config
            .ibkr_oauth_realm
            .as_deref()
            .unwrap_or("limited_poa");
        let header_value = format!(
            "OAuth realm=\"{}\", {}",
            realm,
            params
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(header_value)
    }
}

#[derive(Deserialize)]
struct LiveSessionTokenResponse {
    diffie_hellman_response: String,
    live_session_token_signature: String,
    live_session_token_expiration: u64,
}

fn compute_live_session_token(
    keys: &IbkrKeys,
    consumer_key: &str,
    access_token: &str,
    access_token_secret_b64: &str,
    realm: &str,
    base_url: &str,
) -> Result<(String, u64)> {
    let encrypted_secret = BASE64.decode(access_token_secret_b64.trim()).context("decode access token secret")?;
    let decrypting_key = rsa::pkcs1v15::DecryptingKey::new(keys.encryption_key.clone());
    let prepend_bytes = decrypting_key
        .decrypt(&encrypted_secret)
        .map_err(|e| anyhow::anyhow!("decrypt access token secret: {}", e))?;
    let prepend_hex = hex::encode(&prepend_bytes);

    let dh_random: BigInt = {
        let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes)
    };
    let dh_challenge = keys.dh_generator.modpow(&dh_random, &keys.dh_prime);
    let dh_challenge_hex = format!("{:x}", dh_challenge);
    let dh_challenge_hex = if dh_challenge_hex.len() % 2 == 1 {
        format!("0{}", dh_challenge_hex)
    } else {
        dh_challenge_hex
    };

    let lst_url = format!("{}/oauth/live_session_token", base_url.trim_end_matches('/'));
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let mut oauth_params: BTreeMap<String, String> = BTreeMap::new();
    oauth_params.insert("diffie_hellman_challenge".to_string(), dh_challenge_hex.clone());
    oauth_params.insert("oauth_consumer_key".to_string(), consumer_key.to_string());
    oauth_params.insert("oauth_nonce".to_string(), nonce.clone());
    oauth_params.insert("oauth_signature_method".to_string(), "RSA-SHA256".to_string());
    oauth_params.insert("oauth_timestamp".to_string(), timestamp.to_string());
    oauth_params.insert("oauth_token".to_string(), access_token.to_string());

    let params_str = oauth_params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode_rfc3986(k), percent_encode_rfc3986(v)))
        .collect::<Vec<_>>()
        .join("&");
    let base_string = format!(
        "{}{}&{}&{}",
        prepend_hex,
        "POST",
        percent_encode_rfc3986(&lst_url),
        percent_encode_rfc3986(&params_str)
    );

    use rsa::pkcs1v15::SigningKey;
    let signing_key = SigningKey::<Sha256>::new(keys.signature_key.clone());
    let signature = signing_key.sign(base_string.as_bytes());
    let sig_bytes: &[u8] = signature.as_ref();
    let sig_b64 = percent_encode_rfc3986(&BASE64.encode(sig_bytes));
    oauth_params.insert("oauth_signature".to_string(), sig_b64);
    oauth_params.insert("realm".to_string(), realm.to_string());

    let auth_header = format!(
        "OAuth {}",
        oauth_params
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build http client")?;
    let res = client
        .post(&lst_url)
        .header("Authorization", &auth_header)
        .header("User-Agent", "lianel-stock-service/1.0")
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip, deflate")
        .header("Connection", "keep-alive")
        .header("Host", "api.ibkr.com")
        .send()
        .context("POST live_session_token")?;

    let status = res.status();
    let body = res.text().context("response body")?;
    if !status.is_success() {
        anyhow::bail!("live_session_token failed {}: {}", status, body);
    }

    let data: LiveSessionTokenResponse = serde_json::from_str(&body).context("parse LST response")?;
    let dh_response = data.diffie_hellman_response.trim();
    let dh_response = if !dh_response.is_empty() && !dh_response.starts_with('0') {
        format!("0{}", dh_response)
    } else {
        dh_response.to_string()
    };
    let B = BigInt::from_str_radix(&dh_response, 16).context("parse dh_response")?;
    let K = B.modpow(&dh_random, &keys.dh_prime);

    let mut hex_k = format!("{:x}", K);
    if hex_k.len() % 2 == 1 {
        hex_k = format!("0{}", hex_k);
    }
    let k_bytes = hex::decode(&hex_k).context("K hex to bytes")?;

    let mut mac = HmacSha1::new_from_slice(&k_bytes).context("HMAC-SHA1 key")?;
    mac.update(&prepend_bytes);
    let lst_bytes = mac.finalize().into_bytes();
    let computed_lst = BASE64.encode(lst_bytes);

    let mut verify_mac = HmacSha1::new_from_slice(&BASE64.decode(computed_lst.as_str()).context("decode LST for verify")?)
        .context("HMAC verify")?;
    verify_mac.update(consumer_key.as_bytes());
    let verify_hex = hex::encode(verify_mac.finalize().into_bytes());
    if verify_hex != data.live_session_token_signature {
        anyhow::bail!(
            "LST signature mismatch: got {} expected {}",
            verify_hex,
            data.live_session_token_signature
        );
    }

    Ok((computed_lst, data.live_session_token_expiration))
}
