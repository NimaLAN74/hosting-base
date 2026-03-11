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
use rsa::signature::Signer;
use rsa::traits::{Decryptor, PublicKeyParts};
use rsa::RsaPrivateKey;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    keys: Option<Arc<IbkrKeys>>,
}

struct IbkrKeys {
    dh_prime: BigInt,
    dh_generator: BigInt,
    encryption_key: RsaPrivateKey,
    signature_key: RsaPrivateKey,
}

/// RFC 3986 unreserved characters (A-Za-z0-9-._~) must NOT be encoded in OAuth signature base string.
/// NON_ALPHANUMERIC encodes everything except 0-9 A-Z a-z; we must not encode - . _ ~.
const RFC3986_ENCODE_SET: percent_encoding::AsciiSet =
    percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');

fn percent_encode_rfc3986(s: &str) -> String {
    use percent_encoding::utf8_percent_encode;
    utf8_percent_encode(s, &RFC3986_ENCODE_SET).to_string()
}

impl IbkrOAuthClient {
    pub fn new(config: Arc<AppConfig>) -> Self {
        let keys = Self::load_keys(config.as_ref()).ok().map(Arc::new);
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

        let (dh_prime, dh_generator) = if dh_pem.contains("BEGIN DH PARAMETERS") {
            let dh = openssl::dh::Dh::params_from_pem(dh_pem.as_bytes())
                .context("parse DH PARAMETERS PEM")?;
            let p_bytes = dh.prime_p().to_vec();
            let g_bytes = dh.generator().to_vec();
            (
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &p_bytes),
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &g_bytes),
            )
        } else {
            let dh_key =
                RsaPrivateKey::from_pkcs8_pem(&dh_pem).context("parse DH param as RSA key")?;
            let n_bytes = dh_key.n().to_bytes_be();
            let e_bytes = dh_key.e().to_bytes_be();
            (
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &n_bytes),
                BigInt::from_bytes_be(num_bigint::Sign::Plus, &e_bytes),
            )
        };

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
        let keys = Arc::clone(self.keys.as_ref().context("IBKR keys not loaded")?);
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
            .unwrap_or("test_realm")
            .to_string();
        let base_url = config.ibkr_api_base_url.clone();

        let (token, expires_at_ms) = tokio::task::spawn_blocking(move || {
            compute_live_session_token(
                keys.as_ref(),
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
            .unwrap_or("test_realm");
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

    /// Build HTTP client for IBKR API (optionally skips TLS verify for local Client Portal Gateway).
    pub fn http_client(&self) -> Result<reqwest::Client> {
        let mut b = reqwest::Client::builder().timeout(Duration::from_secs(15));
        if self.config.ibkr_insecure_skip_tls_verify {
            b = b.danger_accept_invalid_certs(true);
        }
        b.build().context("reqwest client")
    }

    /// True when using Gateway with a pre-set session cookie (no OAuth/tickle).
    pub fn use_oauth_for_api(&self) -> bool {
        !self.config.is_ibkr_gateway_cookie_mode()
    }

    /// Call GET /tickle to obtain brokerage session token. Required for /iserver/* (e.g. marketdata/snapshot).
    /// When in Gateway cookie mode, returns the pre-configured session; otherwise OAuth + (SSODH for api.ibkr.com) + tickle.
    /// Returns the session value to send as cookie: `api={session}`.
    pub async fn get_session_for_cookie(&self) -> Result<String> {
        if self.config.is_ibkr_gateway_cookie_mode() {
            return self
                .config
                .ibkr_gateway_session_cookie
                .clone()
                .context("IBKR_GATEWAY_SESSION_COOKIE not set");
        }

        let base_url = self.config.ibkr_api_base_url.trim_end_matches('/');
        if base_url.contains("api.ibkr.com") {
            if let Err(e) = self.ensure_ssodh_connected().await {
                tracing::warn!("SSODH brokerage session step failed (continuing with tickle): {}", e);
            }
        }

        let tickle_url = format!("{}/tickle", base_url);
        let auth = self.sign_request("GET", &tickle_url, None).await?;
        let client = self.http_client()?;
        let resp = client
            .get(&tickle_url)
            .header("Authorization", auth)
            .header("User-Agent", "Console")
            .send()
            .await
            .context("GET /tickle")?;
        let status = resp.status();
        let body = resp.text().await.context("tickle body")?;
        if !status.is_success() {
            anyhow::bail!("tickle failed {}: {}", status, body.trim_start().chars().take(200).collect::<String>());
        }
        let tickle: TickleResponse = serde_json::from_str(&body).context("parse tickle JSON")?;
        Ok(tickle.session)
    }

    /// Establish brokerage session for api.ibkr.com via SSODH init/response (required for market data).
    async fn ensure_ssodh_connected(&self) -> Result<()> {
        let base_url = self.config.ibkr_api_base_url.trim_end_matches('/');
        let lst = self.get_live_session_token().await?;
        let init_url = format!("{}/iserver/auth/ssodh/init", base_url);
        let mac = format!(
            "{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}",
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        );
        let machine_id: String = (0..8)
            .map(|_| {
                let c = rand::random::<u8>();
                if c % 2 == 0 { (b'0' + (c % 10)) as char } else { (b'A' + (c % 26)) as char }
            })
            .collect();
        let form_body = format!(
            "compete=false&locale=en_US&mac={}&machineId={}&username=-",
            mac, machine_id,
        );
        let auth = self.sign_request_post_with_body("POST", &init_url, &form_body).await?;
        let client = self.http_client()?;
        let resp = client
            .post(&init_url)
            .header("Authorization", auth)
            .header("User-Agent", "Console")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body.clone())
            .send()
            .await
            .context("POST ssodh/init")?;
        let status = resp.status();
        let body = resp.text().await.context("ssodh init body")?;
        if !status.is_success() {
            anyhow::bail!("ssodh/init failed {}: {}", status, body.trim_start().chars().take(200).collect::<String>());
        }
        let init_res: SsodhInitResponse = serde_json::from_str(&body).context("parse ssodh init JSON")?;
        let challenge = init_res.challenge.trim();
        let lst_hex = hex::encode(lst.as_bytes());
        let combined_hex = format!("{}{}", challenge, lst_hex);
        let combined_bytes = hex::decode(combined_hex.as_str()).context("combined hex decode")?;
        let r_hash = Sha1::digest(&combined_bytes);
        let r_hex = hex::encode(r_hash);

        let response_url = format!("{}/iserver/auth/ssodh/response", base_url);
        let response_body = format!("response={}", percent_encode_rfc3986(&r_hex));
        let auth_resp = self.sign_request_post_with_body("POST", &response_url, &response_body).await?;
        let resp2 = client
            .post(&response_url)
            .header("Authorization", auth_resp)
            .header("User-Agent", "Console")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(response_body)
            .send()
            .await
            .context("POST ssodh/response")?;
        let status2 = resp2.status();
        let body2 = resp2.text().await.context("ssodh response body")?;
        if !status2.is_success() {
            anyhow::bail!("ssodh/response failed {}: {}", status2, body2.trim_start().chars().take(200).collect::<String>());
        }
        Ok(())
    }

    /// Sign a POST request including form body params in the OAuth signature (for SSODH).
    async fn sign_request_post_with_body(&self, method: &str, url: &str, form_body: &str) -> Result<String> {
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

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("oauth_consumer_key".to_string(), consumer_key.to_string());
        params.insert("oauth_nonce".to_string(), nonce.clone());
        params.insert("oauth_signature_method".to_string(), "HMAC-SHA256".to_string());
        params.insert("oauth_timestamp".to_string(), timestamp.to_string());
        params.insert("oauth_token".to_string(), access_token.to_string());
        for pair in form_body.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let k = percent_encode_rfc3986(k);
                let v = percent_encode_rfc3986(v);
                params.insert(k, v);
            }
        }
        let params_str = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
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

        let mut oauth_params: BTreeMap<&str, String> = BTreeMap::new();
        oauth_params.insert("oauth_consumer_key", consumer_key.to_string());
        oauth_params.insert("oauth_nonce", nonce);
        oauth_params.insert("oauth_signature_method", "HMAC-SHA256".to_string());
        oauth_params.insert("oauth_timestamp", timestamp.to_string());
        oauth_params.insert("oauth_token", access_token.to_string());
        oauth_params.insert("oauth_signature", oauth_signature);
        let realm = self
            .config
            .ibkr_oauth_realm
            .as_deref()
            .unwrap_or("test_realm");
        let header_value = format!(
            "OAuth realm=\"{}\", {}",
            realm,
            oauth_params
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(header_value)
    }
}

#[derive(Deserialize)]
struct TickleResponse {
    session: String,
}

#[derive(Deserialize)]
struct SsodhInitResponse {
    challenge: String,
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
    let sig_bytes: Box<[u8]> = signature.into();
    let sig_b64 = percent_encode_rfc3986(&BASE64.encode(&*sig_bytes));
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
        .header("Content-Length", "0")
        .body("")
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
    let mut k_bytes = hex::decode(&hex_k).context("K hex to bytes")?;
    // IBKR spec: prepend a null byte if K would lack a sign bit (i.e., MSB set).
    // Equivalent to: if bitlen(K) % 8 == 0 then prefix 0x00.
    if !k_bytes.is_empty() && (k_bytes[0] & 0x80) != 0 {
        let mut prefixed = Vec::with_capacity(k_bytes.len() + 1);
        prefixed.push(0u8);
        prefixed.extend_from_slice(&k_bytes);
        k_bytes = prefixed;
    }

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
