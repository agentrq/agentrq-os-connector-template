//! A starting point for a REST API that takes an API key. The host injects
//! the key the user stored as the `X-Api-Key` header, after the request
//! leaves the guest: this code never sees it. Point `DEFAULT_BASE_URL`, the
//! manifest's allowlist and the tools at your API.

use connectors_sdk::http::{Request, Response};
use connectors_sdk::{Connector, ConnectorError, ToolRequest, ToolResponse, host};
use serde::Deserialize;
use serde_json::{Value, json};

connectors_sdk::manifest!("manifest.json");

/// Where the API runs unless the `base_url` input says otherwise.
pub const DEFAULT_BASE_URL: &str = "https://api.example.com";

/// The connector.
#[derive(Debug)]
pub struct ApiKey;

#[derive(Debug, Deserialize)]
struct List {
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Get {
    id: String,
}

#[derive(Debug, Deserialize)]
struct Create {
    title: String,
    notes: Option<String>,
}

impl Connector for ApiKey {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        let base = host::config("base_url").unwrap_or_else(|| DEFAULT_BASE_URL.into());
        answer(&outgoing(&base, &request)?.send()?)
    }
}

connectors_sdk::export_connector!(ApiKey);

/// The request a tool call makes to the API at `base`.
fn outgoing(base: &str, request: &ToolRequest) -> Result<Request, ConnectorError> {
    let base = base.trim_end_matches('/');
    let outgoing = match request.tool.as_str() {
        "listItems" => {
            let input: List = request.json()?;
            Request::get(format!("{base}/items?limit={}", input.limit.unwrap_or(20)))
        }
        "getItem" => {
            let input: Get = request.json()?;
            Request::get(format!("{base}/items/{}", encode(&input.id)))
        }
        "createItem" => {
            let input: Create = request.json()?;
            let body = json!({ "title": input.title, "notes": input.notes });
            Request::post(format!("{base}/items")).json(&body)?
        }
        other => return Err(ConnectorError::NotFound(format!("no tool {other}"))),
    };
    Ok(outgoing.header("accept", "application/json"))
}

/// The tool result of the API's response: its JSON, or a tool error that
/// says what the API answered, so the model can react.
fn answer(response: &Response) -> Result<ToolResponse, ConnectorError> {
    if !(200..300).contains(&response.status) {
        let body = String::from_utf8_lossy(&response.body);
        return Ok(ToolResponse::error(format!(
            "The API answered {}: {body}",
            response.status
        )));
    }
    let body: Value = response.json()?;
    ToolResponse::structured(&json!({ "result": body }))
}

/// Percent-encodes a path segment.
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
        outgoing("https://api.test/", &request(tool, body)).unwrap()
    }

    fn response(status: u16, body: &str) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn plans_each_tool() {
        assert_eq!(
            planned("listItems", "{}").url,
            "https://api.test/items?limit=20"
        );
        assert_eq!(
            planned("listItems", r#"{"limit":5}"#).url,
            "https://api.test/items?limit=5"
        );
        assert_eq!(
            planned("getItem", r#"{"id":"a/b"}"#).url,
            "https://api.test/items/a%2Fb"
        );
        let created = planned("createItem", r#"{"title":"Milk"}"#);
        assert_eq!(created.url, "https://api.test/items");
        let body: Value = serde_json::from_slice(&created.body).unwrap();
        assert_eq!(body, json!({ "title": "Milk", "notes": null }));
        assert_eq!(
            created.headers.last().unwrap(),
            &("accept".to_owned(), "application/json".to_owned())
        );
    }

    #[test]
    fn refuses_bad_input() {
        let missing = outgoing(DEFAULT_BASE_URL, &request("getItem", "{}"));
        assert!(
            matches!(missing, Err(ConnectorError::InvalidInput(_))),
            "{missing:?}"
        );
        let unknown = outgoing(DEFAULT_BASE_URL, &request("nope", "{}"));
        assert_eq!(
            unknown.unwrap_err(),
            ConnectorError::NotFound("no tool nope".into())
        );
    }

    #[test]
    fn maps_responses() {
        let ok = answer(&response(200, r#"{"id":"1"}"#)).unwrap();
        assert_eq!(ok.structured.as_deref(), Some(r#"{"result":{"id":"1"}}"#));
        let refused = answer(&response(401, "bad key")).unwrap();
        assert_eq!(
            refused,
            ToolResponse::error("The API answered 401: bad key")
        );
        let garbled = answer(&response(200, "<html>"));
        assert!(
            matches!(garbled, Err(ConnectorError::Upstream(_))),
            "{garbled:?}"
        );
    }

    #[test]
    fn off_wasm_nothing_is_sent() {
        let sent = ApiKey::call_tool(request("listItems", "{}"));
        assert!(matches!(&sent, Err(ConnectorError::Upstream(m)) if m.contains("wasm32-wasip2")));
    }

    #[test]
    fn manifest_is_embedded() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(
            manifest["permissions"]["http"]["allow"][0],
            DEFAULT_BASE_URL
        );
    }
}
