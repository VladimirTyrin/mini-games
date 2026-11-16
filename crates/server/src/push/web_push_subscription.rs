use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mg_protocol::proto;
use p256::PublicKey;
use p256::elliptic_curve::sec1::ToSec1Point;

use super::{PushTargetError, WebPushEndpoint};

/// A browser's push subscription: where to send, and the keys its messages are encrypted for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WebPushSubscription {
    pub(crate) endpoint: WebPushEndpoint,
    pub(crate) client_key: PublicKey,
    pub(crate) auth_secret: [u8; 16],
}

impl WebPushSubscription {
    pub(crate) fn parse(raw: proto::WebPushSubscription) -> Result<Self, PushTargetError> {
        let endpoint = WebPushEndpoint::parse(raw.endpoint)?;
        let client_key = decode_base64url(&raw.p256dh)
            .and_then(|bytes| PublicKey::from_sec1_bytes(&bytes).ok())
            .ok_or(PushTargetError::ClientKey)?;
        let auth_secret = decode_base64url(&raw.auth)
            .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
            .ok_or(PushTargetError::AuthSecret)?;
        Ok(Self {
            endpoint,
            client_key,
            auth_secret,
        })
    }

    pub(crate) fn to_wire(&self) -> proto::WebPushSubscription {
        proto::WebPushSubscription {
            endpoint: self.endpoint.url().to_owned(),
            p256dh: URL_SAFE_NO_PAD.encode(self.client_key.to_sec1_point(false).as_bytes()),
            auth: URL_SAFE_NO_PAD.encode(self.auth_secret),
        }
    }
}

/// Browsers send unpadded base64url; padding is tolerated.
fn decode_base64url(text: &str) -> Option<Vec<u8>> {
    if text.len() > 256 {
        return None;
    }
    URL_SAFE_NO_PAD.decode(text.trim_end_matches('=')).ok()
}
