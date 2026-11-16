//! Uses a throwaway RSA key generated for these tests only.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::json;

use super::*;

const TEST_PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAzJzWp92gX+PToXBFUhEebVqWKYqq0DKtlsR0BBRReYwbpWfQ
Qd6IIi5gupHITs5q8LGM4jXTGtOq3wK3zaTWrOZtBqt0tcDWDzoY00BfjFSJqTIQ
/WznGd9sAlUFUWN5KnyOTYgg0e/TxDY5EABYTWI6O6iWbWZBQ1CHbnzUnBxIBiZt
rmK6r8zG57MxV3fiLAMf2Att3IIAhQDlbhl7iSRw7r5B0VuCHKI7ZWVEOow6/6+5
6MDfbzXeCRADqaxulll4ml0MJBhyIAaaejtCKJvE/2FhJDZudGglWsSq9DB4nx6N
O2YORPsS2pTL9S5/nx5ToMM5xoWMHcDI0nLUpQIDAQABAoIBAA1szPi+peMkw6H4
B9RWozRxmSCopIR8jr8xrJsH6kCbXMUBhryLqUr02AvlvMrO6F1aiO/esDc2ZCO1
vfGUOzGk/MpPPZ0fRexB15ckxiAi+9VKH6tBFfiwnBmbEnpAmJadDYkwwVaE8dQR
6rSOET0ikMeRTC1pZCbShdBOCHUll9uQ7R3M8zByt/Ah/J1Ka42FsGsHllKsBVuS
WWy8BRZXgh2Keku/UX/6Pwhm686TkSuFMXvmYq5Rddnl/ZmhaK+4ABRcbtoaBBPH
EgM0VEsl9FY3EpS6FU3ddRrynk4fmPcaYbvQCRGiaSbF8DgWq5TzsRyKykJCLdaW
AgpyaYECgYEA5c7bAacSHpq8vr8nhIPf35NyEZWdEpyuqFu8of8kV0XZhb6BfvFS
vuSyeQRAWplipPprrr34uLKqd6DJ22WZYBtPUnP6wY3HZrjkdXSimuRKhPUOT8Fn
NzqZLIeWXPPFUFFf9ExxFbemDwpcBIVSbLHkvFlhssU07XdBjP3t1e0CgYEA4+7a
gNKQIk9LEUasu2VkhZN945b4ZZfMMswr6fKQD+FhwLy0USZta9z7fK9gMmk8pvuZ
edZABR3vaJa4fn63PCLjGyyLiU4bz0jwvj+QHDl3nwxDx5McunGbW2P8dX0XJf/p
aaayAE+vV5FoOpNomIuhiPn90Xr6aE8b5GfmopkCgYEAt5sanWpnbq5AiMHfB9/5
GP8LzwLTORCX9wk8X82islurXFEF029ppfzAnzGHXRtZlehB6p37P034JpX/UNwO
u5t2N7FUMhpdqzOjzDgAxnIJG8tSwOQL3DNkOBCoUOGuzuoFGQGOBETzVub29+Gx
JgjHcLiU9yjajbOPWt9yi00CgYAyuNx2/mSEvhB+b7PQn+eWshO5qcbtFLKkOzsq
fIgCgK04LsYRqPOApVRxkrVdgn20yxiEPGFSwi6cn62+uIQeJcVULr1+eJtHrXul
Wo+/OCqegiQzLVGfS9i+palwm72i/yf6EtZZq2HhosIHBvanu6reyILrMuq2e6RB
V73DQQKBgBUXSic3PDwcDNouH+wgoH+JdG04eCnVZ5u2LbAJD/6DfvwK+0L7apBW
izQ1V9uPWzcVbwM2VWUo+JHIZ6UI003JBFp5tI00wGZoqwQE+Hld122AzxULx8RF
U/n2xtOlUYwZyF569M1Y0Kh/ICKaB+ducchFpBrALny6qDLed/cW
-----END RSA PRIVATE KEY-----";
const TEST_MODULUS: &str = "zJzWp92gX-PToXBFUhEebVqWKYqq0DKtlsR0BBRReYwbpWfQQd6IIi5gupHITs5q8LGM4jXTGtOq3wK3zaTWrOZtBqt0tcDWDzoY00BfjFSJqTIQ_WznGd9sAlUFUWN5KnyOTYgg0e_TxDY5EABYTWI6O6iWbWZBQ1CHbnzUnBxIBiZtrmK6r8zG57MxV3fiLAMf2Att3IIAhQDlbhl7iSRw7r5B0VuCHKI7ZWVEOow6_6-56MDfbzXeCRADqaxulll4ml0MJBhyIAaaejtCKJvE_2FhJDZudGglWsSq9DB4nx6NO2YORPsS2pTL9S5_nx5ToMM5xoWMHcDI0nLUpQ";
const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";

struct StaticKeys;

fn test_keys() -> FetchedKeys {
    let keys = serde_json::from_value(json!({
        "keys": [{ "kty": "RSA", "alg": "RS256", "use": "sig", "kid": "test-kid", "n": TEST_MODULUS, "e": "AQAB" }]
    }))
    .unwrap();
    FetchedKeys {
        keys,
        max_age: Duration::from_secs(3600),
    }
}

impl GoogleKeySource for StaticKeys {
    fn fetch(&self) -> Result<FetchedKeys, KeyFetchError> {
        Ok(test_keys())
    }
}

/// Lets a test hold a fetch in progress: the source reports that it started and then waits.
struct FetchGate {
    started: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

/// Counts fetches; fails when `failing`.
struct CountingKeys {
    fetches: Arc<AtomicUsize>,
    failing: bool,
    gate: Option<FetchGate>,
}

impl GoogleKeySource for CountingKeys {
    fn fetch(&self) -> Result<FetchedKeys, KeyFetchError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            gate.started.send(()).unwrap();
            gate.release.lock().unwrap().recv().unwrap();
        }
        if self.failing {
            return Err(KeyFetchError::Request(ureq::Error::HostNotFound));
        }
        Ok(test_keys())
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn sign(kid: &str, claims: serde_json::Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_owned());
    let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY.as_bytes()).unwrap();
    encode(&header, &claims, &key).unwrap()
}

fn valid_claims() -> serde_json::Value {
    json!({ "iss": "https://accounts.google.com", "aud": CLIENT_ID, "sub": "1234567890", "name": "Ada Lovelace", "exp": now_unix() + 600 })
}

fn verifier() -> GoogleVerifier {
    GoogleVerifier::new(CLIENT_ID.to_owned(), Box::new(StaticKeys))
}

#[test]
fn accepts_valid_token() {
    let identity = verifier()
        .verify(&sign("test-kid", valid_claims()))
        .unwrap();
    assert_eq!(identity.subject, "1234567890");
    assert_eq!(identity.display_name.as_str(), "Ada Lovelace");
}

#[test]
fn rejects_wrong_audience_issuer_expiry_and_key() {
    let mut claims = valid_claims();
    claims["aud"] = json!("someone-else");
    assert!(verifier().verify(&sign("test-kid", claims)).is_err());

    let mut claims = valid_claims();
    claims["iss"] = json!("https://evil.example");
    assert!(verifier().verify(&sign("test-kid", claims)).is_err());

    let mut claims = valid_claims();
    claims["exp"] = json!(now_unix() - 3600);
    assert!(verifier().verify(&sign("test-kid", claims)).is_err());

    assert!(matches!(
        verifier().verify(&sign("other-kid", valid_claims())),
        Err(AuthError::UnknownGoogleKey)
    ));
    assert!(verifier().verify("not-a-jwt").is_err());
}

#[test]
fn parses_cache_control_max_age() {
    assert_eq!(
        parse_max_age("public, max-age=21600, must-revalidate"),
        Some(Duration::from_secs(21600))
    );
    assert_eq!(parse_max_age("no-cache"), None);
}

#[test]
fn failed_fetch_without_keys_backs_off_instead_of_refetching() {
    let fetches = Arc::new(AtomicUsize::new(0));
    let verifier = GoogleVerifier::new(
        CLIENT_ID.to_owned(),
        Box::new(CountingKeys {
            fetches: fetches.clone(),
            failing: true,
            gate: None,
        }),
    );
    let token = sign("test-kid", valid_claims());
    assert!(matches!(
        verifier.verify(&token),
        Err(AuthError::GoogleKeysUnavailable(KeyFetchError::Request(_)))
    ));
    assert!(matches!(
        verifier.verify(&token),
        Err(AuthError::GoogleKeysRetryLater)
    ));
    assert_eq!(fetches.load(Ordering::SeqCst), 1);
}

#[test]
fn concurrent_verifications_share_one_fetch_done_outside_the_lock() {
    let fetches = Arc::new(AtomicUsize::new(0));
    let (started, fetch_started) = mpsc::channel();
    let (release_fetch, release) = mpsc::channel();
    let verifier = Arc::new(GoogleVerifier::new(
        CLIENT_ID.to_owned(),
        Box::new(CountingKeys {
            fetches: fetches.clone(),
            failing: false,
            gate: Some(FetchGate {
                started,
                release: Mutex::new(release),
            }),
        }),
    ));
    let token = sign("test-kid", valid_claims());
    let first = {
        let (verifier, token) = (verifier.clone(), token.clone());
        std::thread::spawn(move || verifier.verify(&token))
    };
    fetch_started.recv().unwrap();
    // The lock is free while the first thread fetches; this one waits for that fetch.
    let second = {
        let (verifier, token) = (verifier.clone(), token.clone());
        std::thread::spawn(move || verifier.verify(&token))
    };
    assert!(verifier.lock().fetch_in_flight);
    release_fetch.send(()).unwrap();
    assert_eq!(first.join().unwrap().unwrap().subject, "1234567890");
    assert_eq!(second.join().unwrap().unwrap().subject, "1234567890");
    assert_eq!(fetches.load(Ordering::SeqCst), 1);
}
