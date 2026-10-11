---
name: build-connector
description: "Build an AgentRQ connector, a WebAssembly (wasm32-wasip2) component written in Rust with connectors-sdk that the `connectors` host serves as an MCP server and a CLI app. Use when the user wants to wrap a REST/HTTP API or service (Gmail, a CRM, an internal API…) as a connector or MCP tool for AgentRQ Connectors, scaffold a connector crate, implement its tools, test it on the host, build the .wasm, or install and try it with `connectors install`. Also use to decide between writing a component and installing an existing remote MCP server by URL."
argument-hint: "[service] [what it should do]"
---

# Build a connector

A connector is a Rust crate compiled to a `wasm32-wasip2` component with
`connectors-sdk`. Its `manifest.json` (embedded at build time) declares the
tools, how the host authenticates outbound requests, and which origins the
code may call. The host validates every call against the schemas, injects
credentials after the request leaves the guest, and serves the connector both
over MCP (`callTool` with `gmail.tools listThreads …`) and as a CLI
(`connectors gmail tools listThreads …`).

If the user named a service or goal, it is in `$ARGUMENTS`.

## 0. Does the service already run an MCP server?

If it does, there is nothing to write: `connectors install https://<its MCP URL>`
makes it a connector, with its sign-in, tools, resources and prompts discovered.
To list it in the catalog, see the `publish-connector` skill. Write a component
only when there is no MCP server, or to keep the code in the sandbox behind an
allowlist. Ask the user if you cannot tell.

No local Rust toolchain, but a `connectors` host with the built-in `wasmrs`
connector? It does the same build on the host: `wasmrs.tools newProject`
(the same templates), `writeFile`, `build` (the compiler's errors come back
with file and line), then `manage.tools installBuilt` with the path the
build returned. The plan below applies either way.

## 1. Plan with the user first

Before writing code, find the API docs and show the user a short plan:

- The endpoints that cover each capability: method, URL, and the OAuth scopes
  or permissions each needs. **Ask before including any endpoint that writes,
  sends or deletes.**
- One tool per capability: a camelCase `name`, a `title`, a one-line
  `description` written for a model, and its input fields.
- How the API authenticates, which picks the template:

| The API uses | Template | Manifest `auth` scheme |
|--------------|----------|------------------------|
| Nothing (pure computation, public data) | `minimal` | none; also no `permissions` if it makes no HTTP calls |
| An API key or personal token | `api-key` | `header` (any header name, optional `prefix`), `bearer`, or `query` |
| A user's own account (sign in with Google, GitHub, Slack…) | `oauth2` | `oauth2` with `authorization_code` (PKCE is always on) |
| A machine-to-machine client id + secret | `oauth2`, then switch the grant | `oauth2` with `client_credentials` |

Wait for the user to confirm the plan, then build it.

## 2. Scaffold

```sh
"${CLAUDE_SKILL_DIR}/scripts/new-connector.sh" <name> --template <minimal|api-key|oauth2> [--dir <parent>]
```

`<name>` is the connector id: `^[a-z][a-z0-9-]{0,62}$`, such as `my-crm`. The
script writes `<parent>/<name>/` with `Cargo.toml` (a standalone crate,
`connectors-sdk` from git), `rust-toolchain.toml` (Rust 1.99 + the
`wasm32-wasip2` target, installed by rustup on first use), `manifest.json`
and `src/lib.rs` copied from the template, already renamed. Templates are also
readable directly in `${CLAUDE_SKILL_DIR}/templates/`.

- Pass `--sdk-path <checkout>/sdk/connectors-sdk` to build against a local
  checkout of github.com/agentrq/connectors instead of git.
- Inside the agentrq/connectors repository itself, do not use the script: copy
  a `templates/<t>` crate, use `connectors-sdk.workspace = true`, and add the
  crate to the workspace `members`, `ALLOWED` in
  `bin/connectors/tests/architecture.rs`, the wasm clippy `-p` list in CI and
  `GUESTS` in `scripts/ci.sh`.

## 3. Write the manifest

Edit `manifest.json`: `title`, `description`, `version`, the `tools`, `auth`,
`inputs`, `permissions.http.allow` and `limits`. Load the `connector-manifest`
skill for the field reference, the auth schemes and the rule codes. The
essentials:

- `name` must equal the crate's connector id; tool names are camelCase and not
  one of `auth`, `help`, `search`, `searchTools`, `tools`, `resources`, `prompts`.
- Every `inputSchema` has an object root and `"additionalProperties": false`.
  Query arguments (`-q`) and headers (`-h`) need a `querySchema` /
  `headersSchema` under `_meta["connectors.agentrq.com/io"]`, or they are refused.
- Set `annotations`: `readOnlyHint` for reads, `destructiveHint` for deletes or
  overwrites, `openWorldHint` for anything that calls out.
- `permissions.http.allow` lists only the API's https origins. The OAuth
  `tokenUrl` does not need to be listed.
- Never put a credential in the manifest or the code. `auth` declares it; the
  user stores it at sign-in; the host injects it.

## 4. Implement the tools

Read `references/sdk.md` for the SDK API. The shape every template follows,
and that keeps the code testable without a network:

```rust
impl Connector for MyCrm {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        let base = host::config("base_url").unwrap_or_else(|| DEFAULT_BASE_URL.into());
        answer(&outgoing(&base, &request)?.send()?)
    }
}
```

- `outgoing(base, request) -> Result<Request, ConnectorError>`: one `match`
  arm per tool, deserialising the body with `request.json::<T>()?` into a
  `#[derive(Deserialize)]` struct and building the `http::Request`. Pure, so
  it is unit-tested.
- `answer(&Response) -> Result<ToolResponse, ConnectorError>`: maps a non-2xx
  status to `ToolResponse::error("The API answered {status}: {body}")` so the
  model can react, and a 2xx JSON body to `ToolResponse::structured(&value)`.
- Unknown tool → `ConnectorError::NotFound(format!("no tool {other}"))`.
- Percent-encode every path segment and query value that comes from input
  (the templates have an `encode` helper). Strip `\r`/`\n` from anything that
  lands in a header or an RFC 822 message.
- Optional `inputs` with a `default`: the default is not stored, so fall back
  in code: `host::config("x").unwrap_or_else(|| DEFAULT.into())`. Read only
  inputs the manifest declares: if you drop the template's base-URL input,
  drop its `host::config` call too.
- Each call gets a fresh instance: statics do not survive between calls.
- Keep the `connectors_sdk::manifest!("manifest.json");` and
  `connectors_sdk::export_connector!(T);` lines.

## 5. Test on the host, then build

```sh
"${CLAUDE_SKILL_DIR}/scripts/check.sh" <crate dir>
```

It runs `cargo test` (everything but the export glue builds for the host;
`host::config` returns `None` and `send()` fails with an error naming
`wasm32-wasip2` there), then `cargo build --release --target wasm32-wasip2`,
and prints the component path: `target/wasm32-wasip2/release/<name with _>.wasm`.

Write tests for each tool's planned request (URL, method, body), for bad
input (`InvalidInput`), for an unknown tool (`NotFound`), and for `answer` on
2xx, 4xx and a non-JSON body. See `references/testing.md`. If the manifest
breaks a rule, the build fails listing every violation with its JSON Pointer;
fix them all, then rebuild.

## 6. Install and try it

Needs the `connectors` binary. If `command -v connectors` finds nothing, see
`references/testing.md` for the Docker alias or building it from source.

For a connector named `my-crm` with a `getItem` tool:

```sh
"${CLAUDE_SKILL_DIR}/scripts/check.sh" my-crm --install
connectors my-crm tools getItem help
printf '%s\n' "$API_KEY" | connectors my-crm tools auth     # header/bearer/query schemes
connectors my-crm tools auth                                # oauth2: opens the browser sign-in
connectors my-crm tools getItem -b '{"id":"42"}'
```

Never ask the user to paste a secret into the chat. Tell them to run the
`auth` command themselves (in Claude Code: `! connectors my-crm tools auth`).
Over MCP, run `connectors serve` and call `callTool` with
`{"command":"my-crm.tools getItem -b '{\"id\":\"42\"}'"}`; the curl form is in
`references/testing.md`.

## 7. Before you call it done

Go through `references/checklist.md`, or delegate a review to the
`connector-reviewer` agent. Then offer to publish it (the `publish-connector`
skill).
