# connectors-sdk reference

The guest-side crate. Everything except the export glue also builds for the
host, so `cargo test` runs your logic without wasm.

```rust
use connectors_sdk::http::{Method, Request, Response};
use connectors_sdk::content::{text_resource, user_prompt};
use connectors_sdk::{Connector, ConnectorError, ToolRequest, ToolResponse, host};

connectors_sdk::manifest!("manifest.json");   // validates + embeds; defines `pub const MANIFEST: &str`
// … impl Connector for T …
connectors_sdk::export_connector!(T);          // once, at the crate root; expands to nothing off wasm
```

## `Connector`

| Method | Required | Returns |
|--------|----------|---------|
| `fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError>` | yes | One tool's result. The host already checked body, query and headers against the schemas. |
| `fn read_resource(uri: &str) -> Result<String, ConnectorError>` | no (default: `NotFound`) | A JSON array of MCP `ResourceContents`; build it with `text_resource(uri, mime, text)`. |
| `fn get_prompt(name: &str, args: &str) -> Result<String, ConnectorError>` | no (default: `NotFound`) | A JSON `GetPromptResult`; build it with `user_prompt(description, text)`. `args` is a JSON object of strings. |
| `fn skills() -> &'static Skills` | no (default: none) | The Agent Skills it serves: `&SKILLS` from `connectors_sdk::skills!("skills")`. |
| `fn list_skills()`, `fn get_skill(uri)`, `fn read_skill_directory(uri)` | no (default: from `skills()`) | Only for skills made on demand: JSON SEP-2640 entries, one entry, a folder's `Resource`s. |

All of them are associated functions (no `self`): each call runs in a fresh
instance, so nothing survives between calls.

## `ToolRequest`

| Item | What it is |
|------|------------|
| `tool: String` | The tool name. |
| `body: String` | The JSON body (`-b`, or `body` in `callTool`). |
| `query: Vec<(String, String)>` | `-q k=v` pairs. Only if the tool has a `querySchema`. |
| `headers: Vec<(String, String)>` | `-h` headers, names lowercased. Only with a `headersSchema`. |
| `json::<T>()` | Deserialises `body`; a mismatch is `ConnectorError::InvalidInput`. |
| `query(k)`, `header(k)` | The first value, as `Option<&str>`. |

In tests: `ToolRequest { tool: "getItem".into(), body: r#"{"id":"1"}"#.into(), ..ToolRequest::default() }`.

## `ToolResponse`

| Constructor | Use it for |
|-------------|------------|
| `ToolResponse::text(s)` | A plain-text result. |
| `ToolResponse::structured(&v)?` | A JSON **object** (else `Internal`), sent as structured content plus the same JSON as text. Wrap arrays: `json!({ "result": list })`. |
| `ToolResponse::error(s)` | The tool ran and failed in a way the model should read and react to: "unknown city", "The API answered 404: …". `isError: true`. |

## `ConnectorError`

`NotFound`, `InvalidInput`, `Upstream`, `Internal` — each takes a `String`.
Use it when the call cannot produce a result at all. It reaches the caller as
a tool error naming the kind (`upstream error: …`). `HttpError` converts into
`Upstream`, so `?` works on `send()`, `json()` and `error_for_status()`.

## `http`

```rust
let response = Request::get(format!("{base}/items/{}", encode(&id)))      // or ::post, ::new(Method::Delete, url)
    .header("accept", "application/json")
    .json(&body)?                                                         // sets content-type, serialises
    .send()?;                                                             // blocking, over wasi:http
let item: Item = response.error_for_status()?.json()?;
```

- `Request { method, url, headers, body }` fields are public: assert on them in tests.
- `Response { status, headers, body }`, plus `header(name)`, `text()?`, `json::<T>()?`, `error_for_status()?`.
- `HttpError`: `InvalidUrl`, `Encode`, `Transport` (refused origin, private address, DNS, TLS), `Status { status, body }`, `Decode`.
- Off wasm, `send()` returns a `Transport` error that names `wasm32-wasip2`; nothing is sent.

What the host does on the way out: it refuses any origin not in
`permissions.http.allow` and any private, loopback or link-local address;
drops any `authorization`, `cookie` or `proxy-*` header the guest set; adds
the credential; refreshes OAuth tokens and retries once after a 401; enforces
`limits.timeoutMs` and `limits.memoryMb` (they can only lower the host's caps,
10 s and 32 MB for server calls by default), and `limits.cli` for calls from
the CLI (caps 2 minutes and 1 GB by default). `ToolRequest::channel()` says
where a call came from (`Channel::Mcp` or `Channel::Cli`, from the
`x-connectors-channel` header the host sets), so a tool can allow the CLI
more work.

## `host`

| Function | Notes |
|----------|-------|
| `host::config(name) -> Option<String>` | A plain `inputs` value the user stored, as text. Secret inputs always return `None`. A manifest `default` is **not** stored: fall back in code. Off wasm: `None`. |
| `host::log(host::Level::Debug, msg)` | `Trace`, `Debug`, `Info`, `Warn`, `Error`. Tagged with the connector; stderr off wasm. Never log bodies that may hold personal data. |

## Files

A tool that takes or returns a file uses a base64 string with
`"contentEncoding": "base64"` in its schema. The CLI then accepts
`-f data=./photo.png` and encodes the file itself; `-o <path>` saves a
base64 output instead of printing it.

## Resources and prompts

Declare them in the manifest's `resources`, `resourceTemplates` and `prompts`,
then implement `read_resource` / `get_prompt` (the `minimal` template has one
of each). They are reached through `callTool` — `<c>.resources read <uri>`,
`<c>.prompts get <name> -b '{"arg":"value"}'` — and as MCP's own
`resources/*` and `prompts/*` (`connectors://<c>/…`, `<c>.<name>`).

## Skills

Agent Skills go in `skills/<name>/SKILL.md` (frontmatter `name`, equal to
the folder's, and `description`), with any files beside it. Set
`"skills": true` in the manifest, then:

```rust
connectors_sdk::skills!("skills");

impl Connector for MyConnector {
    // call_tool …
    fn skills() -> &'static connectors_sdk::skills::Skills {
        &SKILLS
    }
}
```

The macro checks every skill and works out its digests at build time. MCP
2026-07-28 clients get them through `skills/list` as
`skill://<c>/<name>/SKILL.md`; `connectors skills export <c>` copies them
into a local agent's skills folder. Adding a file needs `touch src/lib.rs`.
