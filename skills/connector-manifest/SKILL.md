---
name: connector-manifest
description: "The complete connector manifest.json reference: every top-level field, tools, auth, inputs, permissions, limits, icons, and the listing requirements (icons and instructions are mandatory). Use when writing, editing, or validating a manifest.json."
---

# Manifest reference

Every connector carries a `manifest.json`. The `manifest!` macro embeds it in
the component's `connectors:manifest` custom section, and the host reads it
from there without running the component. It describes the connector to MCP
(name, title, version, icons, tools, resources and prompts), and also says how
the connector authenticates, which inputs the user supplies, which origins it
may call and how long it may run.

A [remote connector](#remote-mcp-servers) has no component: its manifest is
written by `connectors install <url>` from what an existing MCP server lists,
and names the server in `remote`.

The formal definition is [schemas/manifest.schema.json](../schemas/manifest.schema.json).
This page covers every field in that schema, plus the rules the schema cannot
express. `crates/infrastructure/tests/docs.rs` checks that it stays complete
and that its examples are valid.

## How a manifest is checked

Install fails if any check fails, and every violation is reported at once,
each with a JSON Pointer and a rule code.

1. **At build time**, `manifest!` parses the file and runs the domain rules
   below. A broken manifest fails `cargo build`.
2. **At install and load**, the host runs everything again:
   - The whole document is checked against `manifest.schema.json`. Every
     object is closed (`additionalProperties: false`), so a misspelt key is
     an error.
   - The server metadata, and every tool, resource, resource template and
     prompt, is checked against the official MCP `schema.json` of **both**
     2026-07-28 and 2025-11-25, so it can be served on either.
   - Every tool schema must have an object root and use the 2020-12 dialect.
     It may contain only local (`#…`) references, and it must compile.
   - The domain rules apply as well: connector id, semver, tool names,
     duplicates, icon schemes and the HTTP allowlist.

| Rule code | Meaning |
|-----------|---------|
| `manifest-schema` | Does not match `manifest.schema.json` |
| `mcp-schema-2026-07-28`, `mcp-schema-2025-11-25` | An MCP object does not match that revision's schema |
| `connector-id` | `name` is not a valid connector id |
| `semver` | `version` is not a semantic version |
| `reserved-tool-name` | A tool uses a grammar keyword as its name |
| `tool-name-format` | A tool name is not camelCase, or is longer than 64 characters |
| `duplicate-name` | Two tools, prompts or inputs share a name |
| `schema-root` | A tool schema's root is not `"type": "object"` |
| `schema-dialect` | A tool schema's `$schema` is not 2020-12 |
| `remote-ref` | A tool schema has a `$ref` or `$dynamicRef` that does not start with `#` |
| `invalid-json-schema` | A tool schema does not compile |
| `icon-src` | An icon `src` is neither `https:` nor `data:` |
| `insecure-http-allow` | An allowlist entry uses plain `http` without `allowInsecure` |
| `invalid-http-allow` | An allowlist entry is not an origin |
| `remote-url` | `remote.url` is not an http(s) URL, or its origin is not in `permissions.http.allow` |
| `remote-fs` | A remote connector asks for `permissions.fs`: only a component can keep files on the host |
| `remote-commands` | A remote connector asks for `permissions.commands`: only a component can run the host's tools |
| `commands-need-fs` | `permissions.commands` without `permissions.fs` `readwrite`: tools run in the connector's directory |
| `workflow-shape` | A workflow asks for something only a connector has: `tools`, `auth`, `inputs`, `remote`, or `permissions.http`, `fs` or `commands` |
| `workflow-graph` | A workflow's steps do not make a graph: an undeclared `start` or target, a repeated or badly named step, a step no path leads to |
| `model` | Passed every check but does not fit the host's model |

## A complete example

Keys are written in alphanumeric order, at every level of the document:

```json manifest
{
  "$schema": "https://connectors.agentrq.com/schemas/manifest/v1.json",
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "gmail",
  "title": "Gmail",
  "version": "0.1.0",
  "description": "Read and send Gmail messages",
  "websiteUrl": "https://mail.google.com",
  "icons": [
    { "src": "https://www.gstatic.com/images/branding/product/2x/gmail_48dp.png", "mimeType": "image/png", "sizes": ["48x48"] }
  ],
  "tools": [
    {
      "name": "sendEmail",
      "title": "Send email",
      "description": "Send an email from the signed-in account",
      "inputSchema": {
        "type": "object",
        "properties": {
          "to": { "type": "string", "format": "email", "description": "Recipient" },
          "subject": { "type": "string" },
          "text": { "type": "string", "description": "Plain-text body" }
        },
        "required": ["to"],
        "additionalProperties": false
      },
      "annotations": { "destructiveHint": false, "openWorldHint": true }
    },
    {
      "name": "listMessages",
      "title": "List messages",
      "description": "List the newest messages, optionally filtered by label",
      "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
      "outputSchema": {
        "type": "object",
        "properties": { "messages": { "type": "array" } },
        "required": ["messages"]
      },
      "annotations": { "readOnlyHint": true, "openWorldHint": true },
      "_meta": {
        "connectors.agentrq.com/io": {
          "querySchema": {
            "type": "object",
            "properties": {
              "maxResults": { "type": "integer", "minimum": 1, "maximum": 500 },
              "labelIds": { "type": "string", "description": "Comma-separated label ids" }
            },
            "additionalProperties": false
          },
          "headersSchema": {
            "type": "object",
            "properties": { "x-request-id": { "type": "string" } }
          }
        }
      }
    }
  ],
  "resources": [
    { "uri": "gmail://labels", "name": "labels", "title": "Labels", "mimeType": "application/json" }
  ],
  "resourceTemplates": [
    { "uriTemplate": "gmail://messages/{id}", "name": "message", "title": "A message", "mimeType": "message/rfc822" }
  ],
  "prompts": [
    {
      "name": "summarize",
      "title": "Summarize a thread",
      "arguments": [{ "name": "thread", "description": "Thread id", "required": true }]
    }
  ],
  "auth": {
    "required": true,
    "schemes": [
      {
        "type": "oauth2",
        "grants": ["authorization_code"],
        "authorizationUrl": "https://accounts.google.com/o/oauth2/v2/auth",
        "tokenUrl": "https://oauth2.googleapis.com/token",
        "scopes": ["https://www.googleapis.com/auth/gmail.modify"],
        "pkce": true,
        "clientSecretRequired": true,
        "extraAuthParams": { "access_type": "offline", "prompt": "select_account consent" }
      }
    ]
  },
  "inputs": [
    { "name": "client_id", "title": "OAuth client ID", "type": "string" },
    { "name": "page_size", "title": "Default page size", "type": "integer", "default": 20 }
  ],
  "permissions": {
    "http": { "allow": ["https://gmail.googleapis.com"] }
  },
  "limits": { "timeoutMs": 20000, "memoryMb": 64 }
}
```

## Top-level fields

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `$schema` | string | no | If present, exactly `https://connectors.agentrq.com/schemas/manifest/v1.json`. Lets editors validate the file. |
| `manifestVersion` | `1` | yes | The format version of this document. |
| `mcpVersion` | `"2026-07-28"` \| `"2025-11-25"` | yes | The MCP revision the author wrote against. The host validates and serves the connector on both revisions either way. |
| `name` | string | yes | The connector id, used in commands (`gmail.tools`), in the data directory and in `CONNECTORS_<NAME>_*` variables. It must match `^[a-z][a-z0-9-]{0,62}$`: a lowercase letter, then up to 62 lowercase letters, digits or `-`. |
| `title` | string | no | Display name, 1–128 characters. |
| `version` | string | yes | A [semantic version](https://semver.org), such as `1.2.0` or `0.3.0-beta.1`. |
| `description` | string | no | Up to 4096 characters. It appears in `help` and is searched by `searchTools`. |
| `instructions` | string | no | Up to 4096 characters. Guidance for the model using the connector: when to call it, how its tools fit together, and any conventions it follows. The public catalogue lists only connectors that carry instructions. |
| `websiteUrl` | string | no | An `http` or `https` URL. |
| `icons` | array of [icons](#icons) | no | Icons for the connector. |
| `tools` | array of [tools](#tools) | no | What `<c>.tools` lists and calls. |
| `resources` | array of [resources](#resources) | no | Reached with `<c>.resources`. |
| `resourceTemplates` | array of [resource templates](#resource-templates) | no | Listed with the resources. |
| `prompts` | array of [prompts](#prompts) | no | Reached with `<c>.prompts`. |
| `skills` | boolean | no | `true` when the connector serves [Agent Skills](#skills) over MCP. The host only asks connectors that say so. |
| `auth` | [auth](#auth) | no | How the host authenticates the connector's outbound requests. |
| `inputs` | array of [inputs](#inputs) | no | Values the user supplies when signing in. |
| `permissions` | [permissions](#permissions) | no | Which origins the connector may call, whether it keeps files on the host, and which of the host's tools it runs. With no `permissions` it has no network access, no files and no tools. |
| `limits` | [limits](#limits) | no | Lower per-call limits than the host's. |
| `workflow` | [workflow](#workflows) | no | Makes the component a workflow: step functions the host runs one after the other. |
| `remote` | object | no | Only in a [remote connector](#remote-mcp-servers)'s manifest: the MCP server that serves it. A component whose manifest has it is refused. |

The `name`, `title`, `version`, `description`, `websiteUrl` and `icons` fields
together must also be a valid MCP `Implementation`.

## Icons

Used at the top level and on tools, resources, resource templates and prompts.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `src` | string | yes | An `https:` URL or a `data:` URI. Anything else, such as `http:` or `file:`, is refused (`icon-src`). |
| `mimeType` | string | no | For example `image/png` or `image/svg+xml`. |
| `sizes` | array of strings | no | For example `["48x48"]` or `["any"]`. |
| `theme` | `"light"` \| `"dark"` | no | The background the icon is drawn for. |

## Tools

Each tool is an MCP `Tool`.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | camelCase: a lowercase ASCII letter, then ASCII letters or digits (`^[a-z][a-zA-Z0-9]*$`), at most 64 characters. Unique within the connector. It must not be a [reserved name](#reserved-names). |
| `title` | string | no | 1–128 characters. |
| `description` | string | no | Up to 4096 characters. It is searched by `searchTools` and shown in tool help. |
| `inputSchema` | JSON Schema | yes | The request **body** (`-b` or the `body` field). See [tool schemas](#tool-schemas). |
| `outputSchema` | JSON Schema | no | The shape of the tool's structured content. |
| `annotations` | [tool annotations](#tool-annotations) | no | Hints for clients. |
| `icons` | array of [icons](#icons) | no | |
| `_meta` | object | no | Only the keys `connectors.agentrq.com/io` and, in a remote connector, `connectors.agentrq.com/remote` are allowed. |

`_meta["connectors.agentrq.com/io"]` describes the parts of a call that MCP's
single `inputSchema` cannot:

| Field | Type | Notes |
|-------|------|-------|
| `querySchema` | JSON Schema | The `-q` arguments (the `query` field). |
| `headersSchema` | JSON Schema | The `-h` headers (the `headers` field). Write the property names in lowercase, because header names are lowercased before they are checked. |

`_meta["connectors.agentrq.com/remote"]` is written by the host for a
[remote connector](#remote-mcp-servers)'s tool that was renamed to fit the
grammar:

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | The tool's name on the remote server, such as `get_issue` for the tool `getIssue`. Calls use it. |

A tool without a `querySchema` takes no `-q`, and a tool without a
`headersSchema` takes no `-h`; giving them is a violation. Query values given as
`-q k=v` are strings, so they are converted to the boolean, integer or number
that their property asks for, unless that property also accepts strings.

### Tool schemas

`inputSchema`, `outputSchema`, `querySchema` and `headersSchema` are JSON
Schema 2020-12 documents.

- The root must be `"type": "object"` (`schema-root`).
- `$schema` may be left out. If present, it must be
  `https://json-schema.org/draft/2020-12/schema` (`schema-dialect`).
- `$ref` and `$dynamicRef` must be local, such as `#/$defs/address`
  (`remote-ref`). Nothing is ever fetched.
- The schema must compile (`invalid-json-schema`), and `format` is enforced.

Use `"additionalProperties": false` on bodies so a misspelt argument is
reported instead of being passed on silently.

### Tool annotations

| Field | Type | Notes |
|-------|------|-------|
| `title` | string | A display title for the tool. |
| `readOnlyHint` | boolean | The tool does not change anything. |
| `destructiveHint` | boolean | The tool may destroy or overwrite data. |
| `idempotentHint` | boolean | Calling it again with the same input has no further effect. |
| `openWorldHint` | boolean | The tool talks to the outside world. |

These are MCP hints. The host shows them in tool help and does not enforce
them.

## Resources

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `uri` | string | yes | A URI, such as `gmail://labels`. `<c>.resources read <uri>` passes it to the guest's `read-resource`. |
| `name` | string | yes | Non-empty. |
| `title` | string | no | 1–128 characters. |
| `description` | string | no | Up to 4096 characters. |
| `mimeType` | string | no | |
| `size` | integer | no | Size in bytes, 0 or more. |
| `annotations` | [annotations](#annotations) | no | |
| `icons` | array of [icons](#icons) | no | |

## Resource templates

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `uriTemplate` | string | yes | An RFC 6570 template, such as `gmail://messages/{id}`. |
| `name` | string | yes | Non-empty. |
| `title`, `description`, `mimeType`, `annotations`, `icons` | | no | As for [resources](#resources). |

## Annotations

The MCP content annotations used by resources and resource templates.

| Field | Type | Notes |
|-------|------|-------|
| `audience` | array of `"user"` \| `"assistant"` | Who the content is for. No duplicates. |
| `priority` | number | From 0 to 1. |
| `lastModified` | string | An RFC 3339 date-time. |

## Prompts

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | Non-empty and unique among the prompts. |
| `title` | string | no | 1–128 characters. |
| `description` | string | no | Up to 4096 characters. |
| `arguments` | array | no | The arguments `<c>.prompts get <name> -b '{…}'` takes. The host rejects missing required arguments and unknown ones before calling the guest. |
| `icons` | array of [icons](#icons) | no | |

Each argument:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | Non-empty. |
| `title` | string | no | |
| `description` | string | no | |
| `required` | boolean | no | Defaults to `false`. |

## Skills

`"skills": true` says the connector serves
[Agent Skills](https://agentskills.io/specification). A component serves them
through its `connectors:connector/skills` export, which
`connectors_sdk::skills!` writes for it (see the
[authoring guide](authoring.md#skills)); a [remote server](#remote-mcp-servers)
that declares the `io.modelcontextprotocol/skills` extension gets the flag when
it is installed. The skills are not listed in the manifest: they are asked of
the connector when an MCP 2026-07-28 client sends `skills/list`, so a
connector may fetch them on demand.

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "weather",
  "version": "0.1.0",
  "skills": true
}
```

A profile's selection can expose only some skills, by their path in the
connector (`forecasting` for `skill://weather/forecasting/SKILL.md`).

## Auth

`auth` says how the host signs the connector's outbound requests. The guest
never sees the credential: the host injects it after the guest has built the
request (see the [security model](../README.md#security-model)).

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `required` | boolean | no | If `true`, tool calls, resource reads and prompts are refused with "auth required" until a credential is stored. If `false` or absent, calls go out without a credential until the user signs in. |
| `schemes` | array of schemes | yes | At least one. The **first** scheme is the preferred one: `<c>.tools auth` signs in with it, and it decides the implied inputs. |

Each scheme is exactly one of the four shapes below (`oneOf`). When a request
goes out, the host uses the first scheme whose kind matches the stored
credential: an OAuth token goes through an `oauth2` scheme, and an API token
through a `header`, `bearer` or `query` scheme.

### `oauth2`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `type` | `"oauth2"` | yes | |
| `grants` | array of `"authorization_code"` \| `"client_credentials"` | yes | At least one, no duplicates. The **first** grant decides how `auth login` signs in. |
| `authorizationUrl` | https URL | with `authorization_code` | The provider's authorization endpoint. |
| `tokenUrl` | https URL | yes | The token endpoint, used for code exchange, client credentials and refresh. It does not need to be in `permissions.http.allow`. |
| `scopes` | array of strings | no | Requested scopes, sent space-separated. No duplicates. |
| `pkce` | boolean | no | Kept for documentation: the host always sends PKCE S256. |
| `clientSecretRequired` | boolean | no | The OAuth client needs a secret. With `auth.required`, sign-in asks for one. |
| `extraAuthParams` | object of strings | no | Extra authorization request parameters, such as `{"access_type": "offline"}`. They cannot override `state`, `redirect_uri`, `client_id`, `scope`, `response_type` or the PKCE parameters. |
| `inject` | object | no | Where the access token goes. Without it: `Authorization: Bearer <token>`. |
| `resource` | http(s) URL | no | An [RFC 8707](https://www.rfc-editor.org/rfc/rfc8707) resource indicator, sent as `resource` with the authorization, token and refresh requests. MCP servers need it. |
| `registrationUrl` | https URL | no | An [RFC 7591](https://www.rfc-editor.org/rfc/rfc7591) registration endpoint. A sign-in without an OAuth client registers a public one there, so the client id is not asked for. |

`inject`:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `in` | `"header"` \| `"query"` | yes | |
| `name` | string | yes | The header or query parameter name. |
| `prefix` | string | no | Put before the token in a header, such as `"Bearer "`. |

The authorization code flow uses the redirect URI
`<public_base_url>/oauth/callback` on a server, and
`http://127.0.0.1:<port>/oauth/callback` from the CLI. Register both with the
provider. The `iss` of an RFC 9207 response must share an origin with
`authorizationUrl`. Tokens are refreshed when they expire within 60 s, and
once more after an upstream 401.

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "crm",
  "version": "1.0.0",
  "auth": {
    "required": true,
    "schemes": [
      {
        "type": "oauth2",
        "grants": ["client_credentials"],
        "tokenUrl": "https://auth.crm.example.com/oauth/token",
        "scopes": ["contacts.read"],
        "clientSecretRequired": true,
        "inject": { "in": "header", "name": "X-Access-Token" }
      }
    ]
  },
  "permissions": { "http": { "allow": ["https://api.crm.example.com"] } }
}
```

### `header`

An API key in a header of your choice.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `type` | `"header"` | yes | |
| `name` | string | yes | An RFC 9110 header name, 1–128 characters. |
| `prefix` | string | no | Put before the key, such as `"Token "`. Defaults to none. |

### `bearer`

`Authorization: Bearer <token>`, with a token the user pastes in.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `type` | `"bearer"` | yes | No other fields. |

### `query`

A token in a query parameter, `?<param>=<token>`. The host replaces any value
the guest set for that parameter and keeps the others as they were.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `type` | `"query"` | yes | |
| `param` | string | yes | 1–128 characters from `A–Z a–z 0–9 _ . ~ -`. |

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2025-11-25",
  "name": "weather",
  "version": "0.2.0",
  "auth": {
    "required": true,
    "schemes": [
      { "type": "header", "name": "X-Api-Key" },
      { "type": "bearer" },
      { "type": "query", "param": "appid" }
    ]
  },
  "permissions": { "http": { "allow": ["https://api.weather.example.com"] } }
}
```

### Implied inputs

Sign-in asks for these inputs without the manifest declaring them. They come
from the preferred (first) scheme:

| Preferred scheme | Implied inputs |
|------------------|----------------|
| `header`, `bearer`, `query` | `token` (secret), required when `auth.required` |
| `oauth2` | `client_id`, required when `auth.required`; `client_secret` (secret), required when `auth.required` and `clientSecretRequired` |

An entry in `inputs` with one of these names only adds a title or
description, or makes the input required. OAuth clients can also be stored
ahead of time with `connectors init --oauth-client <c>=<id>[:<secret>]`,
`CONNECTORS_<NAME>_CLIENT_ID` / `_CLIENT_SECRET`, or `[oauth.<c>]` in
`config.toml`.

## Inputs

Plain values the user supplies at sign-in, such as an account region or a
base URL. On the CLI they are asked for one by one; on a server, the
`/connect/<c>` page shows them as a form.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | `^[a-z][a-z0-9_]{0,63}$`. Unique among the inputs. |
| `title` | string | no | The label shown to the user. |
| `description` | string | no | Help text. |
| `type` | `"string"` \| `"number"` \| `"integer"` \| `"boolean"` | yes | Only primitive values. |
| `secret` | boolean | no | Asked for without echo, or as a password field, and never readable by the guest. |
| `required` | boolean | no | Sign-in fails without it. |
| `default` | string, number or boolean | no | Shown to the user. An input with a default is not asked for, and counts as given when it is required. |

The guest reads plain inputs with `connectors_sdk::host::config("<name>")`,
which returns the stored value as text. A default is not stored, so the guest
should fall back to it itself:
`host::config("base_url").unwrap_or_else(|| DEFAULT.into())`. Secret inputs
always return `None`.

## Permissions

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `http` | object | no | Outbound HTTP. |
| `allow` (inside `http`) | array | yes | The origins the connector may call. Each entry is a string or an object. |

A string entry is an `https` origin: scheme, host and optional port, with no
path, query, fragment or user info, such as `https://api.example.com` or
`https://api.example.com:8443`. A host of `*.example.com` matches subdomains
at any depth but not `example.com` itself. The port must match, and an
omitted port means the scheme's default (443, or 80 for http). The
object form is needed only for plain http:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `origin` | string | yes | As above, but may be `http://`. |
| `allowInsecure` | boolean | no | Must be `true` for an `http://` origin (`insecure-http-allow`). |

The allowlist never unlocks private, loopback or link-local addresses, even
when a listed name resolves to one. A request to any other origin fails in the
guest with a transport error.

### Files

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `fs` | object | no | A directory of its own on the host. |
| `access` (inside `fs`) | `"read"` \| `"readwrite"` | yes | `read` lists and reads files; `readwrite` also creates, changes and deletes them. |

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "permissions": { "fs": { "access": "readwrite" } }
}
```

The connector sees its directory as `/data` and nothing else of the host:
`std::fs` works on paths under `/data`, and every path is resolved inside it,
so `..`, absolute paths and symlinks cannot leave it. There is one directory
for each organization, user, profile and connector,
`<CONNECTORS_FS_ROOT>/o-<org>/u-<user>/<profile>/<connector>/`, made `0700`.
Writes stop at the space limit (10 GB by default, `CONNECTORS_FS_QUOTA_MB`,
and per connector in the web UI) with `insufficient-space`, as on a full disk
(`std::fs`, writing through a stream, reports it as an I/O error).

Files are off on a host unless its operator turns them on
(`CONNECTORS_FS_ACCESS`); until then such a connector is not installed, and
one installed earlier runs no tool (`denied: filesystem access is turned off
on this host`); its resources, prompts and skills are still served, with no
`/data` directory. Each install must be acknowledged by the user: the web UI
asks, `connectors install <file> --allow-fs` says so on the command line,
and the API takes `?acknowledge=fs` (upload) or `"acknowledge": ["fs"]`
(catalog). A remote connector cannot ask for files (`remote-fs`).

### Machine tools

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `commands` | array | no | Tools of the host's machine the connector runs. Needs `fs` with `readwrite`. |
| `name` (inside an entry) | string | yes | The tool's name in the host's setup, `^[a-z0-9][a-z0-9_-]{0,31}$`, once each. |
| `kind` (inside an entry) | `"cli"` \| `"mcp"` | yes | `cli` runs it as a command line; `mcp` talks to it as an MCP server on stdio. |

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "shots",
  "version": "0.1.0",
  "permissions": {
    "fs": { "access": "readwrite" },
    "commands": [
      { "name": "ffmpeg", "kind": "cli" },
      { "name": "obscura", "kind": "mcp" },
      { "name": "rustwright", "kind": "mcp" }
    ]
  }
}
```

The connector calls them through `connectors_sdk::machine` (WIT
`connectors:connector/machine`):

```rust
use connectors_sdk::machine;

// A command line: arguments after those the host's setup gives the tool.
let clip = machine::run("ffmpeg", &["-i", "/data/in.mp4", "/data/out.webm"], b"")?;
// An MCP server: one JSON-RPC request, its `result` back as JSON.
let page = machine::mcp(
    "obscura",
    "tools/call",
    r#"{"name":"browser_navigate","arguments":{"url":"https://example.com"}}"#,
)?;
let shot = machine::mcp(
    "rustwright",
    "tools/call",
    r#"{"name":"browser_take_screenshot","arguments":{}}"#,
)?;
```

A tool runs in the connector's directory for the profile, the one it sees
as `/data`: an argument (or an `=` value, or an MCP string) that starts with
`/data` names the real path, and output names it `/data` again. On Linux,
Landlock lets the tool write only there and read only the system's
directories, its program's install prefix and the directories its operator
listed; it gets no environment but `PATH`, `HOME`, `TMPDIR` and the proxy
variables, and cannot signal other processes. Its only way out is the host's
filtering proxy (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`; `{proxy}` in the
host's arguments for a tool is its address, e.g. obscura's
`--proxy http://{proxy}`), the one TCP port it may connect to: localhost,
loopback, private and other non-public addresses are refused. A command line tool is killed, with every process it
started, at the call's time limit, and may print up to 16 MiB on each of
stdout and stderr. An MCP server is started on first use (the host sends
`initialize` and answers the server's own requests with "method not
found"), kept for the profile, and stopped after 10 minutes unused. Before a
tool runs, a directory past its space limit refuses it.

The host's operator offers tools by name (`CONNECTORS_COMMANDS_ACCESS`,
`CONNECTORS_COMMANDS`). Installing a connector checks that each tool it
lists is offered with the same kind and that its program is on the host,
and is refused otherwise (`toolbox needs machine tools this host cannot run:
obscura (not offered)`). Each install must then be acknowledged:
`connectors install <file> --allow-fs --allow-commands`, or `commands` in
the API's `acknowledge`. A remote connector cannot ask for tools
(`remote-commands`).

## Remote MCP servers

`connectors install https://mcp.example.com/mcp` (or **Discover › Remote MCP** in the
web UI, or `POST /api/v1/remotes`) makes an existing MCP server a connector.
The host asks the server what it offers, writes a manifest for it to
`<data>/o-<org>/connectors/<name>/remote.json`, and from then on proxies its tools,
resources and prompts through `callTool` and the CLI like any other
connector's. The manifest is checked by the same rules as a component's.

- `name` is the one given with `--name`, or one made from the server's
  `serverInfo.name`, else its host (`mcp.linear.app` gives `linear`).
- `title`, `version`, `description`, `websiteUrl` and `icons` come from
  `serverInfo`. A version that is not semver is read as far as it goes
  (`1.2` is `1.2.0`), else it is `0.0.0`.
- `permissions.http.allow` holds only the server's origin, and `remote.url`
  the server's URL. That origin is trusted like a host-supplied entry: it may
  be plain http, or a private or loopback address, so `http://127.0.0.1:3000/mcp`
  works. Every other origin follows the usual rules.
- `auth` comes from the server's answer without a credential. A 401 with
  [RFC 9728](https://www.rfc-editor.org/rfc/rfc9728) protected resource
  metadata, and [RFC 8414](https://www.rfc-editor.org/rfc/rfc8414) or OpenID
  Connect metadata of its authorization server, gives an `oauth2` scheme with
  the authorization code grant, the challenge's scopes (else those of the
  resource), `resource` set to the server and, if the server takes
  registrations, `registrationUrl`. A 401 without that metadata gives a
  `bearer` scheme. A server that answers needs no `auth`.
- Tool names that do not fit the grammar are renamed: camelCased
  (`get_issue` becomes `getIssue`), suffixed when reserved (`search` becomes
  `searchTool`) or taken (`getIssue2`), with the server's name kept in
  `_meta`. Unknown fields are dropped, long texts are cut, and a `$schema`
  other than 2020-12 is dropped, since servers often say draft-07 for
  schemas that read the same. An entry that still breaks a rule is left out,
  and the install says which and why.
- A server that wants a sign-in before it lists anything gets its tools when
  a profile signs in. `connectors refresh <name>` asks again.

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "tracker",
  "title": "Tracker",
  "version": "2.1.0",
  "tools": [
    {
      "name": "getIssue",
      "description": "Gets one issue.",
      "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } } },
      "_meta": { "connectors.agentrq.com/remote": { "name": "get_issue" } }
    }
  ],
  "auth": {
    "required": true,
    "schemes": [
      {
        "type": "oauth2",
        "grants": ["authorization_code"],
        "authorizationUrl": "https://auth.tracker.example.com/authorize",
        "tokenUrl": "https://auth.tracker.example.com/token",
        "scopes": ["issues:read"],
        "pkce": true,
        "resource": "https://mcp.tracker.example.com/mcp",
        "registrationUrl": "https://auth.tracker.example.com/register"
      }
    ]
  },
  "permissions": { "http": { "allow": ["https://mcp.tracker.example.com"] } },
  "remote": { "url": "https://mcp.tracker.example.com/mcp" }
}
```

| Field of `remote` | Type | Required | Notes |
|-------|------|----------|-------|
| `url` | http(s) URL | yes | The server's Streamable HTTP endpoint. No user info or fragment, and its origin must be in `permissions.http.allow` (`remote-url`). |

```json invalid-manifest remote-url
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "tracker",
  "version": "1.0.0",
  "permissions": { "http": { "allow": ["https://api.tracker.example.com"] } },
  "remote": { "url": "https://mcp.tracker.example.com/mcp" }
}
```

How the host talks to the server is in the `mcp-compatibility` skill.

## Several connectors in one repository

The catalog lists GitHub repositories with the `agentrq-connector` topic. A
repository with a single connector needs nothing more: its latest release
carries one `.wasm` asset, whose embedded manifest is checked at install.

A repository that publishes several connectors **must** have a
`connectors-manifest.json` at its root, and its release must name each asset
`<name>.wasm` after the connector's `name`. Without the index, a release with
more than one `.wasm` asset cannot be installed. The formal definition is
[schemas/connectors-manifest.schema.json](../schemas/connectors-manifest.schema.json).
The index's `connectors` array lists the manifests in alphanumeric order by
path, and keys are in alphanumeric order here too.

```json connectors-manifest
{
  "$schema": "https://connectors.agentrq.com/schemas/connectors-manifest/v1.json",
  "manifestVersion": 1,
  "connectors": [
    { "manifest": "connectors/drive/manifest.json" },
    { "manifest": "connectors/gmail/manifest.json" }
  ],
  "bundles": [
    {
      "name": "google",
      "title": "Google Workspace",
      "description": "Mail and files in one go",
      "icons": [{ "src": "data:image/svg+xml;base64,PHN2Zy8+" }],
      "connectors": ["drive", "gmail"]
    }
  ]
}
```

- `manifestVersion` is `1`.
- `connectors` lists 1–100 entries. Each `manifest` is the path of a
  connector's `manifest.json` relative to the repository root: letters,
  digits, `-`, `_`, `.` and `/`, ending in `.json`, at most 256 characters,
  with no empty, `.` or `..` segment. A path may be listed only once.
- Every linked manifest is checked exactly as an install checks it. One that
  is missing, invalid, or reuses the `name` of a connector listed before it is
  left out of the catalog; an index that breaks the rules above hides the
  whole repository.
- Searches read the index at the default branch and list each connector on
  its own, with the title, description, version and first icon of its
  manifest. Installing reads it again at the release's tag.
- `bundles` is optional: up to 20 groups of the listed connectors that people
  install together. Each has a `name` (1–32 lowercase letters and digits,
  unique within the index), an optional `title` shown in the UI (1–32
  characters, the name if left out), an optional `description` (at most 4096
  characters) and `icons` (`https:` or `data:`, as a manifest's), and
  `connectors`: 1–100 distinct connector `name`s. A bundle that breaks these
  rules hides the whole repository, like any other index error.
- Searches list each bundle as its own result next to the connectors, keeping
  only the connectors the index really publishes (a bundle left with none is
  not listed). **Install bundle** installs the ones not yet installed and
  groups them as one of the user's bundles of the same name, which a profile
  then turns on or off at once (see the README's "Bundles").

## Remote MCP servers in the catalog

A repository can also publish a remote MCP server, so that people find it in
the catalog instead of typing its URL. Give the repository the
`agentrq-remote-connector` topic and put a `remote-connector.json` at its
root:

```json remote-connector
{
  "url": "https://mcp.tracker.example.com/mcp",
  "name": "tracker",
  "title": "Tracker",
  "description": "Issues and projects in Tracker."
}
```

- `url` is the only required field: the server's Streamable HTTP endpoint,
  an `https:` URL without a fragment.
- `name` is the connector name to install it as (the rules of a manifest's
  `name`); without it, the name is made from the URL, as when adding a
  server by hand.
- `title` (1–128 characters) and `description` (1–4096) are what the catalog
  shows; without them, the repository's name and description.
- No other field is allowed. A file that is missing or breaks these rules
  keeps the repository out of the catalog.

Searches read the file at the default branch, and list the server on the
**Remote MCP** tab of Discover. Adding it does what adding its URL by hand
does: the host asks the server for its tools and how it signs in, and writes
the connector's manifest from that (see [Remote MCP servers](#remote-mcp-servers)).
Nothing is downloaded from the repository.

## Workflows

A component whose manifest has a `workflow` block is a **workflow**, not a
connector: it holds only logic, and chains the tool calls of other
connectors. It has no tools of its own, no auth, no inputs and no network,
files or machine tools (`workflow-shape`); it only handles results. The host
runs it (see the [authoring guide](authoring.md#9-workflows)) and adds five
tools to it: `startRun` (answers with a run id at once; a run may take up to
an hour), `getRun`, `listRuns`, `cancelRun` and `sendEvent` (an event a step
may wait for). A profile enables a workflow
like a connector, and a run reaches only the tools that profile exposes.
The web UI lists workflows in their own section, with their steps as a
graph and as text, and the runs that are going with the step each branch is
at.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `start` | string | yes | The step that is given the input of a run. |
| `input` | JSON Schema | no | The shape of a run's input, shown in the `startRun` tool. |
| `steps` | array of steps | yes | 1 to 64 step functions. |

Each step:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | The function the component routes on: lowercase letters, digits and underscores, starting with a letter, at most 64. |
| `title` | string | no | Display name. |
| `description` | string | no | What the step does. |
| `ok` | array of step names | no | The steps its ok path may lead to. |
| `err` | array of step names | no | The steps its error path may lead to. |
| `retries` | integer 0 to 3 | no | How many times a call this step asked for is made again when it fails, after 1 s, 2 s, 4 s. Default 0: a failed call goes to `catch` at once. |

The host checks the graph when it loads the component (`workflow-graph`):
`start` and every `ok` and `err` target are declared, names are unique, and
every step can be reached from `start`. Cycles are allowed; a run stops after
100 steps.

A step is called with the run, the result of the call before it (the run's
input for the start step) or its error, and the `state` the step before
passed on. It answers `ok` or `err`, and either may name calls to make next
(several run in parallel, each as a branch of its own) and where each
result goes (`then`, and optionally `catch` for a failed call), or end the
branch with a value. The answer must follow the manifest: a call's `then` has
to be in the step's `ok` list when it answers ok and in `err` when it answers
err, and `catch` in `err`. A run fails on the first step or call that fails
with no error path to take.

Instead of calls, an answer may `wait`: `{"event"?, "timeoutSecs"?, "then",
"catch"?}`. With an `event` the branch parks until `sendEvent` delivers it and
`then` gets its payload as the result; `timeoutSecs` (up to an hour) gives up
after that long, and the timeout goes to `catch` (an error) or fails the run.
With only `timeoutSecs` the branch sleeps and `then` gets null. A wait obeys
the same path rules as a call: `then` in `ok` (or `err` if the answer is err),
`catch` in `err`, and it is not mixed with calls or a value.

```json manifest
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "digest",
  "version": "0.1.0",
  "workflow": {
    "start": "fetch",
    "input": { "type": "object", "properties": { "city": { "type": "string" } } },
    "steps": [
      { "name": "fetch", "title": "Fetch the forecast", "ok": ["summarize"], "err": ["report"] },
      { "name": "summarize", "title": "Summarize", "err": ["report"] },
      { "name": "report", "title": "Report the failure" }
    ]
  }
}
```

```json invalid-manifest workflow-graph
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "digest",
  "version": "0.1.0",
  "workflow": {
    "start": "fetch",
    "steps": [
      { "name": "fetch", "ok": ["summarize"] },
      { "name": "summarize" },
      { "name": "orphan" }
    ]
  }
}
```

```json invalid-manifest workflow-shape
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "digest",
  "version": "0.1.0",
  "permissions": { "http": { "allow": ["https://example.com"] } },
  "workflow": { "start": "fetch", "steps": [{ "name": "fetch" }] }
}
```

## Limits

| Field | Type | Notes |
|-------|------|-------|
| `timeoutMs` | integer, 1–600000 | Wall-clock time per call. |
| `memoryMb` | integer, 1–4096 | Memory per linear memory. |
| `cli` | object: `timeoutMs`, `memoryMb` | Limits for calls from the `connectors` CLI instead of the two above. |

Limits can only **lower** the host's caps. Calls from the server (MCP, the
web UI) are capped at 10 s and 32 MB by default (`CONNECTORS_CALL_TIMEOUT_MS`,
`CONNECTORS_CALL_MEMORY_MB`); calls from the CLI, which runs on the user's
own machine, at 2 minutes and 1 GB (`CONNECTORS_CLI_CALL_TIMEOUT_MS`,
`CONNECTORS_CLI_CALL_MEMORY_MB`). The CLI uses `cli` when given, else
`timeoutMs` and `memoryMb`. A higher value is accepted and has no effect.

```json
{ "limits": { "timeoutMs": 5000, "memoryMb": 32, "cli": { "timeoutMs": 120000, "memoryMb": 1024 } } }
```

Every tool call carries the header `x-connectors-channel: mcp` or `cli`,
which the host sets itself (a caller's copy is dropped), so a connector can
allow the CLI more work: `ToolRequest::channel()` in the SDK.

## Reserved names

These tool names are grammar keywords, compared without regard to case:

`auth`, `help`, `search`, `searchTools`, `tools`, `resources`, `prompts`

A tool with one of these names is refused (`reserved-tool-name`), because
`gmail.tools auth` must always mean "sign in to gmail":

```json invalid-manifest reserved-tool-name
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "tools": [{ "name": "search", "inputSchema": { "type": "object" } }]
}
```

Use a more specific name, such as `searchNotes`.

Connector names are not reserved, but a connector named like a static CLI
command (`serve`, `init`, `install`, `refresh`, `remove`, `list`, `call`, `upstream`,
`skills`, `completions` or `help`) gets no `connectors <name> …` command of its own. Reach it with
`connectors call '<name>.tools'`. `gmail.help` always reaches a connector's
overview, even for a connector named `help` or `connectors`.

More examples of what is refused:

```json invalid-manifest tool-name-format
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "tools": [{ "name": "send_note", "inputSchema": { "type": "object" } }]
}
```

```json invalid-manifest remote-ref
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "tools": [{
    "name": "addNote",
    "inputSchema": { "type": "object", "properties": { "note": { "$ref": "https://example.com/note.json" } } }
  }]
}
```

```json invalid-manifest insecure-http-allow
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "permissions": { "http": { "allow": ["http://notes.example.com"] } }
}
```

```json invalid-manifest manifest-schema
{
  "manifestVersion": 1,
  "mcpVersion": "2026-07-28",
  "name": "notes",
  "version": "0.1.0",
  "permission": { "http": { "allow": ["https://notes.example.com"] } }
}
```
