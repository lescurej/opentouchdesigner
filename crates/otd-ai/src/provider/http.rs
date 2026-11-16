use super::types::{Provider, Request};
use crate::keys::{Key, Keys, redact};

/// Pull the assistant's text out of whichever shape came back.
pub fn extract_text(provider: Provider, body: &serde_json::Value) -> Result<String, String> {
    // A provider that returns 200 with an error object in it is common
    // enough to check for first, whatever the shape.
    if let Some(error) = body.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(message.to_string());
    }

    let text = match provider {
        Provider::Anthropic => body
            .get("content")
            .and_then(|c| c.as_array())
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default(),
        // Never reached: `complete` sends these to `cli` before there is a
        // body to read. Explicit rather than swept into the arm below, so
        // that a wire shape is never invented for a provider that has none.
        Provider::ClaudeCode | Provider::Codex => {
            return Err(format!("{} has no wire format", provider.label()));
        }
        Provider::OpenAi | Provider::OpenRouter => body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string(),
    };

    if text.trim().is_empty() {
        // A refusal or a length stop reads as an empty reply; say which.
        let reason = finish_reason(provider, body);
        return Err(match reason {
            Some(r) => format!("the model returned nothing (stopped: {r})"),
            None => "the model returned nothing".into(),
        });
    }
    Ok(text)
}

fn finish_reason(provider: Provider, body: &serde_json::Value) -> Option<String> {
    let value = match provider {
        Provider::Anthropic => body.get("stop_reason"),
        _ => body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("finish_reason")),
    };
    value.and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Send a request and return the assistant's text.
///
/// Blocking. Call it from a worker thread.
pub fn complete(request: &Request, key: &Key, keys: &Keys) -> Result<String, String> {
    // The two that are not a URL. Dispatched before anything HTTP-shaped
    // happens, so no key, agent, or endpoint is consulted for them.
    if !request.provider.needs_key() {
        return crate::cli::complete(request);
    }

    let Some(endpoint) = request.provider.endpoint() else {
        return Err(format!("{} has no endpoint", request.provider.label()));
    };
    if key.is_empty() {
        return Err(format!(
            "no API key for {} — paste one, or set {}",
            request.provider.label(),
            request
                .provider
                .env_var()
                .unwrap_or("it in the environment")
        ));
    }

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(request.timeout))
        // Handle non-2xx ourselves. The default turns a 401 into an error
        // that has thrown the body away, and the body is where the provider
        // says *which* thing was wrong — worth far more than the number.
        .http_status_as_error(false)
        .build()
        .new_agent();

    let mut call = agent.post(endpoint);
    call = match request.provider {
        Provider::Anthropic => call
            .header("x-api-key", key.expose())
            .header("anthropic-version", "2023-06-01"),
        _ => call.header("authorization", &format!("Bearer {}", key.expose())),
    };
    if request.provider == Provider::OpenRouter {
        // OpenRouter attributes traffic by these, and asks that clients send
        // them. Neither identifies the user.
        call = call
            .header("http-referer", "https://github.com/Ar9av/opentouchdesigner")
            .header("x-title", "OpenTouchDesigner");
    }

    let result = call.send_json(request.body());

    // Every path out of here goes through `redact`: providers quote the
    // request back in error bodies often enough to matter.
    let mut response =
        result.map_err(|e| redact(&format!("could not reach the provider: {e}"), keys))?;
    let status = response.status().as_u16();

    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| redact(&format!("could not read the reply: {e}"), keys))?;

    if !(200..300).contains(&status) {
        return Err(redact(&explain_status(status, &text), keys));
    }

    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| redact(&format!("the reply was not JSON: {e}"), keys))?;
    extract_text(request.provider, &body).map_err(|e| redact(&e, keys))
}

/// Turn a failed response into something worth reading.
///
/// The provider's own message first when there is one — "invalid x-api-key"
/// and "your credit balance is too low" are different problems that both
/// arrive as 401-shaped disappointment — then what to do about it.
fn explain_status(code: u16, body: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message").or(Some(e)))
                .map(|m| match m.as_str() {
                    Some(s) => s.to_string(),
                    None => m.to_string(),
                })
                .or_else(|| v.get("message").and_then(|m| m.as_str()).map(String::from))
        })
        .unwrap_or_else(|| body.chars().take(200).collect());

    let advice = match code {
        401 | 403 => "check the key, and that it is for this provider",
        404 => "check the model name — this provider may not have it",
        413 => "the request was too large; try a smaller network or prompt",
        429 => "rate limited or out of credit; wait, or check billing",
        500..=599 => "the provider is having trouble; try again",
        _ => "the provider refused the request",
    };
    if detail.trim().is_empty() {
        format!("{code}: {advice}")
    } else {
        format!("{code}: {} — {advice}", detail.trim())
    }
}
