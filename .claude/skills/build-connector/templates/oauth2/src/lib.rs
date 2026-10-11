//! A starting point for a REST API with an OAuth 2.0 sign-in, shaped like
//! Gmail: it lists and reads threads and saves drafts. The host runs the
//! sign-in (authorization code with PKCE), stores and refreshes the token,
//! and adds `Authorization: Bearer …` to every request after it leaves the
//! guest: this code never sees it.
//!
//! To use it for another service, change the manifest's `auth` scheme
//! (URLs and scopes), its allowlist and the tools, then `outgoing`.

use connectors_sdk::http::{Request, Response};
use connectors_sdk::{Connector, ConnectorError, ToolRequest, ToolResponse, host};
use serde::Deserialize;
use serde_json::{Value, json};

connectors_sdk::manifest!("manifest.json");

/// Where the API is unless the `api_base` input says otherwise.
pub const DEFAULT_API_BASE: &str = "https://gmail.googleapis.com";

/// The connector.
#[derive(Debug)]
pub struct OAuth2;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListThreads {
    query: Option<String>,
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetThread {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDraft {
    to: String,
    subject: String,
    body: String,
    thread_id: Option<String>,
}

impl Connector for OAuth2 {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        let base = host::config("api_base").unwrap_or_else(|| DEFAULT_API_BASE.into());
        answer(&outgoing(&base, &request)?.send()?)
    }
}

connectors_sdk::export_connector!(OAuth2);

/// The request a tool call makes to the API at `base`.
fn outgoing(base: &str, request: &ToolRequest) -> Result<Request, ConnectorError> {
    let api = format!("{}/gmail/v1/users/me", base.trim_end_matches('/'));
    let outgoing = match request.tool.as_str() {
        "listThreads" => {
            let input: ListThreads = request.json()?;
            let mut url = format!(
                "{api}/threads?maxResults={}",
                input.max_results.unwrap_or(10)
            );
            if let Some(query) = input.query.filter(|query| !query.is_empty()) {
                url = format!("{url}&q={}", encode(&query));
            }
            Request::get(url)
        }
        "getThread" => {
            let input: GetThread = request.json()?;
            Request::get(format!("{api}/threads/{}?format=full", encode(&input.id)))
        }
        "createDraft" => {
            let input: CreateDraft = request.json()?;
            let mut message = json!({ "raw": base64url(rfc822(&input).as_bytes()) });
            if let Some(thread) = input.thread_id {
                message["threadId"] = json!(thread);
            }
            Request::post(format!("{api}/drafts")).json(&json!({ "message": message }))?
        }
        other => return Err(ConnectorError::NotFound(format!("no tool {other}"))),
    };
    Ok(outgoing.header("accept", "application/json"))
}

/// The tool result of the API's response: its JSON, or a tool error that
/// says what the API answered, so the model can react (a 403 usually means
/// a missing scope).
fn answer(response: &Response) -> Result<ToolResponse, ConnectorError> {
    if !(200..300).contains(&response.status) {
        let body = String::from_utf8_lossy(&response.body);
        return Ok(ToolResponse::error(format!(
            "The API answered {}: {body}",
            response.status
        )));
    }
    let body: Value = response.json()?;
    ToolResponse::structured(&body)
}

/// A plain-text RFC 822 message. Header values lose any line break, so the
/// model cannot add headers of its own.
fn rfc822(draft: &CreateDraft) -> String {
    let line = |value: &str| value.replace(['\r', '\n'], " ");
    format!(
        "To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}",
        line(&draft.to),
        line(&draft.subject),
        draft.body
    )
}

/// Base64url without padding, as Gmail's `raw` field wants it.
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}

/// Percent-encodes a path segment or query value.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, body: &str) -> ToolRequest {
        ToolRequest {
            tool: tool.into(),
            body: body.into(),
            ..ToolRequest::default()
        }
    }

    fn planned(tool: &str, body: &str) -> Request {
        outgoing("https://mail.test/", &request(tool, body)).unwrap()
    }

    fn response(status: u16, body: &str) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    const API: &str = "https://mail.test/gmail/v1/users/me";

    #[test]
    fn plans_each_tool() {
        assert_eq!(
            planned("listThreads", "{}").url,
            format!("{API}/threads?maxResults=10")
        );
        let searched = planned(
            "listThreads",
            r#"{"query":"is:unread from:ada","maxResults":5}"#,
        );
        assert_eq!(
            searched.url,
            format!("{API}/threads?maxResults=5&q=is%3Aunread%20from%3Aada")
        );
        assert_eq!(
            planned("listThreads", r#"{"query":""}"#).url,
            format!("{API}/threads?maxResults=10")
        );
        assert_eq!(
            planned("getThread", r#"{"id":"t1"}"#).url,
            format!("{API}/threads/t1?format=full")
        );

        let draft = planned(
            "createDraft",
            r#"{"to":"ada@example.com","subject":"Hi","body":"Hello"}"#,
        );
        assert_eq!(draft.url, format!("{API}/drafts"));
        let body: Value = serde_json::from_slice(&draft.body).unwrap();
        let raw = "To: ada@example.com\r\nSubject: Hi\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nHello";
        assert_eq!(
            body,
            json!({ "message": { "raw": base64url(raw.as_bytes()) } })
        );
        let reply = planned(
            "createDraft",
            r#"{"to":"a@b.c","subject":"Re","body":"x","threadId":"t1"}"#,
        );
        let body: Value = serde_json::from_slice(&reply.body).unwrap();
        assert_eq!(body["message"]["threadId"], "t1");
    }

    #[test]
    fn headers_cannot_be_injected() {
        let draft = CreateDraft {
            to: "a@b.c\r\nBcc: eve@evil.test".into(),
            subject: "Hi\nX: y".into(),
            body: "line\r\nline".into(),
            thread_id: None,
        };
        let message = rfc822(&draft);
        assert!(
            message.starts_with("To: a@b.c  Bcc: eve@evil.test\r\nSubject: Hi X: y\r\n"),
            "{message}"
        );
        assert!(message.ends_with("\r\n\r\nline\r\nline"));
    }

    #[test]
    fn encodes_base64url() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn refuses_bad_input() {
        let missing = outgoing(DEFAULT_API_BASE, &request("getThread", "{}"));
        assert!(
            matches!(missing, Err(ConnectorError::InvalidInput(_))),
            "{missing:?}"
        );
        let unknown = outgoing(DEFAULT_API_BASE, &request("nope", "{}"));
        assert_eq!(
            unknown.unwrap_err(),
            ConnectorError::NotFound("no tool nope".into())
        );
    }

    #[test]
    fn maps_responses() {
        let ok = answer(&response(200, r#"{"threads":[]}"#)).unwrap();
        assert_eq!(ok.structured.as_deref(), Some(r#"{"threads":[]}"#));
        let refused = answer(&response(403, "insufficient scope")).unwrap();
        assert_eq!(
            refused,
            ToolResponse::error("The API answered 403: insufficient scope")
        );
        let garbled = answer(&response(200, "<html>"));
        assert!(
            matches!(garbled, Err(ConnectorError::Upstream(_))),
            "{garbled:?}"
        );
    }

    #[test]
    fn off_wasm_nothing_is_sent() {
        let sent = OAuth2::call_tool(request("listThreads", "{}"));
        assert!(matches!(&sent, Err(ConnectorError::Upstream(m)) if m.contains("wasm32-wasip2")));
    }

    #[test]
    fn manifest_is_embedded() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["auth"]["schemes"][0]["type"], "oauth2");
        assert_eq!(
            manifest["permissions"]["http"]["allow"][0],
            DEFAULT_API_BASE
        );
    }
}
