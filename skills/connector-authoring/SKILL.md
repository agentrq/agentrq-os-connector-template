---
name: connector-authoring
description: "Write an AgentRQ connector end to end: plan the tool surface, scaffold the Rust crate, implement tools with connectors-sdk, build the wasm32-wasip2 component, install and test it on the host. Use when creating a new connector or working through the full authoring flow."
---

# Writing a connector

A connector is a WebAssembly component built for `wasm32-wasip2` with the
`connectors-sdk` crate. This guide builds `weather`, a connector with one tool
that calls an HTTP API with an API key. It then builds it, installs it, and
tests it from the CLI and over MCP.

For complete, working connectors see the built-in
[apps/basic-finance](../apps/basic-finance), [apps/math](../apps/math) and
[apps/time](../apps/time), which have no network, many tools and a resource, and
[examples/httpbin](../examples/httpbin), which makes outbound HTTP with an
injected API key. For production-grade, published connectors, see
[agentrq-os-connectors](https://github.com/agentrq/agentrq-os-connectors).
Every manifest field is
described in the `connector-manifest` skill.

No Rust toolchain at hand? The built-in `wasmrs` connector writes and
builds a connector on the host for a model: `wasmrs.tools newProject`,
`writeFile`, `build`, then `manage.tools installBuilt`
([README](../README.md#the-built-in-wasmrs-connector-rust-to-wasm)).

The built-in `manage` connector serves this guide and ready-to-copy starter
templates (`manage.resources`), and its `createConnector` prompt walks an
assistant through these steps for a given service. In Claude Code, the
[agentrq-connectors plugin](../plugins/claude/README.md) does the same with
skills (`/plugin marketplace add agentrq/connectors`, then
`/plugin install agentrq-connectors@agentrq`).

If the service already runs an MCP server, there is nothing to write:
`connectors install https://<its MCP URL>` makes it a connector (see
[Remote MCP servers](../README.md#remote-mcp-servers)). Write a component when
there is no server, or to keep the code in the sandbox with an allowlist.

## 1. What you need

- Rust 1.99 or newer, with the target: `rustup target add wasm32-wasip2`.
- The `connectors` binary (see the [README](../README.md#quick-start)).

Nothing else: there is no `cargo component` step and no `wasm-tools` step.
`wasm32-wasip2` builds a component directly.

## 2. Create the crate

```sh
cargo new --lib weather
cd weather
```

`Cargo.toml`:

```toml
[package]
name = "weather"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for the component, rlib for host tests

[dependencies]
connectors-sdk = { git = "https://github.com/agentrq/connectors" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Inside this repository, use `connectors-sdk.workspace = true` instead, and add
the crate in three places: the workspace `members`, the `ALLOWED` table in
`bin/connectors/tests/architecture.rs`, and the wasm clippy `-p` list in
`.github/workflows/ci.yml`.

## 3. Write the manifest

`manifest.json`, next to `Cargo.toml`:

```json manifest
{
  "$schema": "https://connectors.agentrq.com/schemas/manifest/v1.json",
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "weather",
  "title": "Weather",
  "version": "0.1.0",
  "description": "Current weather for a city",
  "tools": [
    {
      "name": "current",
      "title": "Current weather",
      "description": "The current temperature and conditions in a city",
      "inputSchema": {
        "type": "object",
        "properties": {
          "city": { "type": "string", "minLength": 1, "description": "City name, e.g. Lisbon" }
        },
        "required": ["city"],
        "additionalProperties": false
      },
      "annotations": { "readOnlyHint": true, "openWorldHint": true },
      "_meta": {
        "connectors.agentrq.com/io": {
          "querySchema": {
            "type": "object",
            "properties": { "units": { "type": "string", "enum": ["metric", "imperial"] } },
            "additionalProperties": false
          }
        }
      }
    }
  ],
  "auth": {
    "required": true,
    "schemes": [{ "type": "header", "name": "X-Api-Key" }]
  },
  "permissions": { "http": { "allow": ["https://api.weather.example.com"] } },
  "limits": { "timeoutMs": 10000 }
}
```

The fields that matter most:

- **`inputSchema`** describes the request body, which is `-b` on the command
  line or `body` in `callTool`. The host validates the body before your code
  runs, so you can deserialise it without checking it again.
- **`_meta["connectors.agentrq.com/io"]`** describes `-q` query arguments and
  `-h` headers. A tool without a `querySchema` accepts no `-q`.
- **`auth`** declares how the host authenticates your requests. A `header`
  scheme makes sign-in ask the user for a `token`. The host stores it,
  encrypted, and adds it as `X-Api-Key` to every outgoing request. Your code
  never sees it.
- **`permissions.http.allow`** lists the only origins your code may call.
- Tool names are camelCase and must not be one of the
  `reserved names` (see the `connector-manifest` skill) (`auth`, `help`, `search`,
  `searchTools`, `tools`, `resources`, `prompts`).

## 4. Implement the connector

`src/lib.rs`:

```rust
use connectors_sdk::http::Request;
use connectors_sdk::{Connector, ConnectorError, ToolRequest, ToolResponse, host};
use serde::{Deserialize, Serialize};

// Checks manifest.json at build time and embeds it in the component.
connectors_sdk::manifest!("manifest.json");

const API: &str = "https://api.weather.example.com";

pub struct Weather;

#[derive(Deserialize)]
struct CurrentInput {
    city: String,
}

#[derive(Deserialize, Serialize)]
struct Report {
    temperature: f64,
    conditions: String,
}

impl Connector for Weather {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        match request.tool.as_str() {
            "current" => current(&request),
            other => Err(ConnectorError::NotFound(format!("no tool {other}"))),
        }
    }
}

fn current(request: &ToolRequest) -> Result<ToolResponse, ConnectorError> {
    let input: CurrentInput = request.json()?;
    let units = request.query("units").unwrap_or("metric");
    host::log(host::Level::Debug, &format!("weather for {}", input.city));

    let response = Request::get(url(&input.city, units))
        .header("accept", "application/json")
        .send()?; // the host adds X-Api-Key on the way out
    if response.status == 404 {
        return Ok(ToolResponse::error(format!("unknown city {}", input.city)));
    }
    let report: Report = response.error_for_status()?.json()?;
    ToolResponse::structured(&report)
}

fn url(city: &str, units: &str) -> String {
    let city: String = city
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("{API}/v1/current?city={city}&units={units}")
}

// Exports Weather as the component's `guest` interface. Off wasm it
// expands to nothing, so `cargo test` works on the host.
connectors_sdk::export_connector!(Weather);
```

The SDK at a glance:

| Item | What it is |
|------|------------|
| `Connector` | The trait to implement. `call_tool` is required. `read_resource` and `get_prompt` default to "not found". |
| `ToolRequest` | `tool`, `body` (JSON text), `query` and `headers` (pairs, header names in lowercase), plus `json::<T>()`, `query(k)` and `header(k)`. |
| `ToolResponse::text(s)` | A successful result with one text block. |
| `ToolResponse::structured(&v)` | Structured content (a JSON object), plus the same JSON as a text block. Pair it with an `outputSchema`. |
| `ToolResponse::error(s)` | A tool error (`isError: true`). Use it when the tool ran and failed in a way the model should see, such as "unknown city". |
| `ConnectorError` | `NotFound`, `InvalidInput`, `Upstream` or `Internal`, for when the call could not produce a result. It reaches the caller as a tool error that names the kind. `HttpError` converts into it, so `?` works. |
| `http::Request` | A blocking HTTP client over `wasi:http`: `get`, `post`, `new(Method, url)`, `.header()`, `.body()`, `.json(&v)?`, `.send()?`. |
| `http::Response` | `status`, `headers`, `body`, plus `header(name)`, `text()?`, `json()?` and `error_for_status()?`. |
| `host::config(name)` | A plain (non-secret) input the user stored, as text. Secret inputs always return `None`. |
| `host::log(level, msg)` | Writes to the host's log, tagged with the connector. |
| `content::text_resource` | Builds the JSON that `read_resource` returns. |
| `content::user_prompt` | Builds the JSON that `get_prompt` returns. |
| `manifest!("manifest.json")` | Validates and embeds the manifest, and defines `pub const MANIFEST: &str`. The path is relative to `Cargo.toml`. |
| `export_connector!(T)` | Exports `T`. Call it once, at the crate root. |

What the host does for you:

- It validates the body, the query and the headers against your schemas
  before `call_tool` runs.
- It enforces the timeout and memory limits. Each call gets a fresh instance,
  so statics do not survive between calls.
- It refuses any origin that is not on the allowlist, and any private
  address. You see this as `HttpError::Transport`.
- It removes any `authorization`, `cookie` or `proxy-*` header you set, and
  adds the credential.
- It refreshes OAuth tokens, and retries once after an upstream 401.

### Files

For a tool that takes or returns a file, use a base64 string with
`"contentEncoding": "base64"` in its schema, as the `file` tool of the test
connector in [fixtures/echo](../fixtures/echo) does. The CLI then accepts
`-f data=./photo.png` for an input property and encodes the file itself. For
output, return an image, audio or embedded `resource` block (with `blob`), or
mark the `outputSchema` property the same way. `-o <path>` then saves the
file instead of printing base64.

### Resources and prompts

Declare them in the manifest's `resources`, `resourceTemplates` and `prompts`,
then implement the two optional methods (see `apps/math`, or `fixtures/echo`
for a prompt):

```rust
use connectors_sdk::content::{text_resource, user_prompt};

impl Connector for Weather {
    // fn call_tool(...) as above

    fn read_resource(uri: &str) -> Result<String, ConnectorError> {
        match uri {
            "weather://about" => Ok(text_resource(uri, "text/plain", "Data from api.weather.example.com")),
            _ => Err(ConnectorError::NotFound(format!("no resource {uri}"))),
        }
    }

    fn get_prompt(name: &str, args: &str) -> Result<String, ConnectorError> {
        let args: serde_json::Value = serde_json::from_str(args)
            .map_err(|e| ConnectorError::InvalidInput(e.to_string()))?;
        match name {
            "packing" => Ok(user_prompt("What to pack", &format!("What should I pack for {}?", args["city"]))),
            _ => Err(ConnectorError::NotFound(format!("no prompt {name}"))),
        }
    }
}
```

These are reached through `callTool`: `weather.resources read weather://about`
and `weather.prompts get packing -b '{"city":"Lisbon"}'`, and as MCP's own
`resources/*` and `prompts/*`, namespaced as `connectors://weather/about` and
`weather.packing`.

### Skills

A connector can ship [Agent Skills](https://agentskills.io/specification):
folders with a `SKILL.md` (YAML frontmatter with `name` and `description`,
then Markdown) and any files it points to. The host serves them to MCP
2026-07-28 clients through the `io.modelcontextprotocol/skills` extension
(SEP-2640), as `skill://weather/<skill>/…`, and `connectors skills export
weather` copies them into a local agent's skills folder.

Put each skill in its own folder under `skills/`, named like its `name`:

```text
weather/
├── Cargo.toml
├── manifest.json        # with "skills": true
├── skills/
│   └── forecasting/
│       ├── SKILL.md
│       └── references/stations.md
└── src/lib.rs
```

Then embed them and hand them to the SDK:

```rust
connectors_sdk::skills!("skills");

impl Connector for Weather {
    // fn call_tool(...) as above

    fn skills() -> &'static connectors_sdk::skills::Skills {
        &SKILLS
    }
}
```

`skills!` checks every skill at build time (the frontmatter is YAML with a
`name` equal to the folder's and a `description`; at most 512 files and
16 MiB a skill) and works out each file's digest, so the guest only looks
things up. Set `"skills": true` in the manifest, or the host never asks.
Adding a file needs a rebuild of the crate (`touch src/lib.rs`); editing one
rebuilds on its own. A `SKILL.md` can also be a resource:
`connectors_sdk::skills::body()` drops its frontmatter (see `apps/math`).

Skills fetched from the service on demand implement `list_skills`,
`get_skill` and `read_skill_directory` instead, return their entries with
`"resources": "dynamic"`, and serve their `skill://` files from
`read_resource`.

## 5. Test on the host

Everything except the export glue builds for the host, so plain `cargo test`
exercises your logic. Aim for 100% line and function coverage
(`cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100`): test
every tool, every input validation, and every error branch. `host::config`
returns `None` there, and `host::log`
writes to stderr. Keep HTTP behind a function that builds the `Request`, as
`examples/httpbin` does, and test that function:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_the_city() {
        assert_eq!(
            url("São Paulo", "metric"),
            "https://api.weather.example.com/v1/current?city=S%C3%A3o%20Paulo&units=metric"
        );
    }

    #[test]
    fn unknown_tool_is_not_found() {
        let request = ToolRequest { tool: "nope".into(), body: "{}".into(), ..ToolRequest::default() };
        assert!(matches!(Weather::call_tool(request), Err(ConnectorError::NotFound(_))));
    }

    #[test]
    fn manifest_is_embedded() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["name"], "weather");
    }
}
```

## 6. Build

```sh
cargo build --release --target wasm32-wasip2
ls target/wasm32-wasip2/release/weather.wasm
```

Hyphens in the crate name become underscores in the file name
(`my-crm` → `my_crm.wasm`). If the manifest breaks a rule, the build fails
with every violation and its JSON Pointer. The JSON Schema checks, against
`manifest.schema.json` and the MCP schemas, run when you install.

## 7. Install and try it

```sh
connectors install target/wasm32-wasip2/release/weather.wasm
connectors weather
connectors weather tools
connectors weather tools current help
printf '%s\n' "$WEATHER_API_KEY" | connectors weather tools auth
connectors weather tools current -b '{"city":"Lisbon"}' -q units=metric
connectors weather tools auth status
```

`api.weather.example.com` does not exist, so the last call ends in
`upstream error: request failed: … DnsError`. Point `API` at a real service,
or at one you run, to see a result. A `-q units=kelvin` is refused before
your code runs, because the `querySchema` allows only `metric` and `imperial`.

`install` validates the manifest and copies the component into
`<data>/o-00000000001/connectors/weather/component.wasm`. Installing again replaces it. A
running `connectors serve` watches that directory, so it picks up an install,
a replacement or a `connectors remove weather` without a restart. Calls that
are already running finish on the old version. If a replacement fails
validation, the old version stays loaded.

The same commands work through `callTool`:

```callTool
weather.tools
weather.tools current help
weather.tools current -b '{"city":"Lisbon"}' -q units=metric
weather.searchTools 'temperature', 3
weather.tools auth
```

## 8. Test over MCP

Start a server and send `callTool` with curl. This is a 2026-07-28 request,
which carries its version in `_meta` and in headers:

```sh
connectors serve &
curl -s http://127.0.0.1:8080/mcp \
  -H 'content-type: application/json' -H 'accept: application/json' \
  -H 'mcp-protocol-version: 2026-07-28' -H 'mcp-method: tools/call' -H 'mcp-name: callTool' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"callTool",
       "arguments":{"command":"weather.tools current help"},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

A 2025-11-25 client sends the same call with no `_meta`. The server keeps no
sessions, so it needs no `initialize` first:

```sh
curl -s http://127.0.0.1:8080/mcp \
  -H 'content-type: application/json' -H 'accept: application/json' \
  -H 'mcp-protocol-version: 2025-11-25' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"callTool",
       "arguments":{"command":"weather.tools current help"}}}'
```

To check conformance as CI does, run
[mcp-spec-test](https://www.npmjs.com/package/@hasmcp/mcp-spec-test) against
the server on both revisions:

```sh
for rev in 2026-07-28 2025-11-25; do
  MCP_DISABLE_TELEMETRY=1 npx -y @hasmcp/mcp-spec-test@0.1.5 -u http://127.0.0.1:8080/mcp \
    --spec-version "$rev" --tool-args '{"callTool":{"command":"weather.tools"}}'
done
```

`scripts/conformance.sh` does this for the examples, with and without a token.

## 9. Workflows

A connector answers calls. A **workflow** chains them: it is a component of
*step functions* that the host runs one after another, each given what the
last call returned, so it can fetch from one connector and write to another
without a model in between. It has no network of its own; it only handles
results. Start one from the `workflow` template (`wasmrs newProject
{"template": "workflow"}`, or copy `templates/workflow`).

The manifest has a `workflow` block instead of `tools` (see
the `Workflows` section of the `connector-manifest` skill: the `start` step and, for each step, the
steps its ok path and its error path may lead to. `manifest!` checks the
graph when the crate compiles.

A step is a function that returns a `Result`, like a monad: `Ok` takes the ok
path, `Err` the error path, and `?` turns a result that does not fit into the
error path.

```rust
use connectors_sdk::workflow::{self, Failure, Next, Step, StepFn, StepResult};
use serde_json::json;

const STEPS: &[(&str, StepFn)] = &[("fetch", fetch), ("digest", digest), ("report", report)];

impl Connector for Digest {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        workflow::route(&request, STEPS)
    }
}

fn fetch(step: Step) -> StepResult {
    let city: String = step.result_at("/city")?; // the run's input, for the start step
    Ok(Next::new()
        .call("weather", "forecast", json!({"city": city}), "digest") // the result goes to `digest`
        .call("news", "headlines", json!({"city": city}), "digest")   // in parallel
        .catch("report"))                                             // a failed call goes to `report`
}

fn digest(step: Step) -> StepResult {
    if step.result.is_null() {
        return Err(Failure::new("nothing came back"));  // no calls: the run fails
    }
    Ok(Next::done(json!({"digest": step.result})))      // ends this branch with a value
}
```

- `Step` has `run` (its `id` and `input`), `result` (the call before it
  returned, null on the error path), `error` (why the call before it failed,
  on the error path) and `state`, which a step passes on with
  `.state(value)` and the next steps read with `step.state_as::<T>()`.
- Several `.call`s run in parallel, each as a branch of its own; a run ends
  when every branch has. Its output is the value of the one branch, or a
  list of the values of several.
- Every `then` and `catch` has to be in the step's `ok` or `err` list: the
  host refuses the answer otherwise and the run fails.
- A run stops after an hour, 100 steps, or 16 MiB in one payload; at most 16
  calls per step. Runs going at once are limited per profile (16), user (64)
  and organization (1000) by default. A run that was going when its host
  stopped is marked failed within about a minute.
- A step whose calls fail may be tried again: `"retries": 2` (at most 3) in its
  manifest entry makes the call again after 1 s, 2 s, then 4 s, before the
  error goes to `catch`. A step that is not safe to repeat leaves it out (the
  default).
- A step may wait instead of calling: `Next::new().wait("paid",
  "ship").timeout(300).catch("refund")` parks the branch until the event
  `paid` is sent to the run (the `sendEvent` tool, `POST
  /api/v1/workflow-runs/{id}/events` or the web UI), and `ship` gets its
  payload as `step.result`; `.sleep(60, "ship")` waits a minute. A wait is
  listed in the manifest like a call: `ship` in `ok`, `refund` in `err`.
- Test the functions with plain `cargo test`: build a `ToolRequest` whose body
  is a `Step` as JSON and read the `structured` answer, as the template does.

Install it like a connector. `startRun {"input": …}` answers with a run id at
once; `getRun {"runId"}` shows which step each branch is at (or what it
waits for), what has happened and the output; `cancelRun` stops it;
`sendEvent {"runId", "event", "payload"?}` wakes a branch that waits. `cron.schedule` can start a
workflow on a schedule: `connector` is the workflow and `tool` is `startRun`.
The web UI lists workflows in their own section, as a graph and as text, and
shows the runs that are going with the step each branch is at.


## 10. Checklist

- [ ] Tool names are camelCase, unique and not reserved.
- [ ] Every body schema sets `"additionalProperties": false`.
- [ ] Every origin you call is in `permissions.http.allow`, using https.
- [ ] No credential is handled in code; `auth` declares it.
- [ ] A failure the model should read is a `ToolResponse::error`; a
      `ConnectorError` is for calls that cannot produce a result.
- [ ] Optional inputs fall back to their `default` in code, because
      `host::config` returns only stored values.
- [ ] `cargo test` passes on the host, and the component builds for
      `wasm32-wasip2`.
