# Testing a connector

## Unit tests on the host

Keep the request-building and response-mapping functions pure and test them;
`send()` cannot reach the network off wasm. A test module that covers a tool
completely:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn request(tool: &str, body: &str) -> ToolRequest {
        ToolRequest { tool: tool.into(), body: body.into(), ..ToolRequest::default() }
    }

    fn response(status: u16, body: &str) -> Response {
        Response { status, headers: Vec::new(), body: body.as_bytes().to_vec() }
    }

    #[test]
    fn plans_each_tool() {
        let planned = outgoing("https://api.test/", &request("getItem", r#"{"id":"a/b"}"#)).unwrap();
        assert_eq!(planned.url, "https://api.test/items/a%2Fb");      // encoded, no double slash
        let created = outgoing("https://api.test", &request("createItem", r#"{"title":"Milk"}"#)).unwrap();
        let body: Value = serde_json::from_slice(&created.body).unwrap(); // compare parsed JSON, never strings
        assert_eq!(body, json!({ "title": "Milk", "notes": null }));
    }

    #[test]
    fn refuses_bad_input() {
        let missing = outgoing("https://api.test", &request("getItem", "{}"));
        assert!(matches!(missing, Err(ConnectorError::InvalidInput(_))), "{missing:?}");
        let unknown = outgoing("https://api.test", &request("nope", "{}"));
        assert_eq!(unknown.unwrap_err(), ConnectorError::NotFound("no tool nope".into()));
    }

    #[test]
    fn maps_responses() {
        assert!(!answer(&response(200, r#"{"id":"1"}"#)).unwrap().is_error);
        assert_eq!(answer(&response(404, "gone")).unwrap(), ToolResponse::error("The API answered 404: gone"));
        assert!(matches!(answer(&response(200, "<html>")), Err(ConnectorError::Upstream(_))));
    }

    #[test]
    fn manifest_is_embedded() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["name"], "my-crm");
    }
}
```

Also test any helper that escapes input: percent-encoding, header values with
`\r\n`, base64.

## Getting the `connectors` binary

Check `command -v connectors` first. Otherwise, either:

- **Docker** (no Rust or Node needed). The CLI is the image's entrypoint:

  ```sh
  alias connectors='docker run --rm -i -v connectors-data:/data -v "$PWD:/work:ro" agentrq/connectors:latest'
  connectors install /work/target/wasm32-wasip2/release/my_crm.wasm
  ```

  Paths are inside the container: run from the crate directory and use
  `/work/…`. An alias does not reach `check.sh --install`; run the commands
  by hand.
- **From source**: clone github.com/agentrq/connectors and
  `cargo build --release -p connectors` (its build script builds the web UI,
  so it needs Node and npm), then put `target/release` on `PATH`.

Use a throwaway data directory while testing so the user's real one is left
alone: `export CONNECTORS_DATA_DIR="$(mktemp -d)"`.

## The CLI

```sh
connectors install target/wasm32-wasip2/release/my_crm.wasm   # validates the manifest; replaces an older install
connectors my-crm                                              # help: tools, resources, prompts, auth
connectors my-crm tools
connectors my-crm tools getItem help                           # schema, annotations, examples
connectors my-crm tools auth status
connectors my-crm tools getItem -b '{"id":"42"}'
connectors my-crm tools listItems -q limit=5                   # only with a querySchema
connectors my-crm searchTools 'invoice', 3
connectors my-crm resources read my-crm://readme
connectors my-crm prompts get welcome -b '{"name":"Ada"}'
connectors remove my-crm
```

`--json` prints the MCP `CallToolResult`. Exit codes: 0 ok, 1 tool or command
error, 2 usage error, 3 the connector needs `auth`.

Sign-in is the user's to do, because it takes their secret:

- `header`, `bearer`, `query` schemes: `printf '%s\n' "$KEY" | connectors my-crm tools auth`,
  or run it interactively and paste at the prompt.
- `oauth2` `authorization_code`: store the OAuth client first
  (`connectors init --oauth-client my-crm=<client id>`, secret in
  `CONNECTORS_MY_CRM_CLIENT_SECRET`), then `connectors my-crm tools auth`
  opens a browser and listens on `http://127.0.0.1:<port>/oauth/callback`
  (register that redirect URI with the provider).

In Claude Code, suggest `! connectors my-crm tools auth` so the user types the
secret into their own terminal, never into the chat.

## Over MCP

```sh
connectors serve &      # http://127.0.0.1:8080/mcp, no auth on loopback
curl -s http://127.0.0.1:8080/mcp \
  -H 'content-type: application/json' -H 'accept: application/json' \
  -H 'mcp-protocol-version: 2025-11-25' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"callTool",
       "arguments":{"command":"my-crm.tools getItem -b '\''{\"id\":\"42\"}'\''"}}}'
```

MCP commands use dots (`my-crm.tools getItem`); the CLI uses spaces
(`connectors my-crm tools getItem`). A running `serve` picks up an install,
a replacement or a removal without a restart. Stop the server when done.

## Errors you will see

| Message | Cause |
|---------|-------|
| `cargo build` lists violations with JSON Pointers | The manifest breaks a domain rule; see the `connector-manifest` skill's rule codes. |
| `install` fails with `manifest-schema` / `mcp-schema-…` | A misspelt or extra key (every object is closed), or an MCP shape error. |
| `upstream error: request failed: response: ErrorCode::HttpRequestDenied` | The origin is missing from `permissions.http.allow` (check the URL the code built, and any base-URL input), or it resolves to a private address. |
| `upstream error: request failed: … DnsError` | The host name does not resolve. |
| `<name> needs authentication: run <name>.tools auth` (exit 3) | `auth.required` is true and nothing is stored yet: the user signs in. |
| A `-q` or `-h` is refused before the code runs | The tool has no `querySchema` / `headersSchema`, or the value breaks it. |
| `rustc … is not supported` | Rust older than 1.99: keep the scaffolded `rust-toolchain.toml`. |
