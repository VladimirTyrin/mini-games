use std::time::Duration;

use super::PushError;

/// HTTP client for push services. Error statuses come back as responses, so their bodies can be
/// logged.
pub(crate) fn push_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .build()
        .new_agent()
}

pub(crate) fn rejection(
    service: &'static str,
    mut response: ureq::http::Response<ureq::Body>,
) -> PushError {
    const MAX_DETAIL_CHARS: usize = 300;
    let status = response.status().as_u16();
    let detail = response
        .body_mut()
        .read_to_string()
        .unwrap_or_default()
        .chars()
        .take(MAX_DETAIL_CHARS)
        .collect();
    PushError::Rejected {
        service,
        status,
        detail,
    }
}
