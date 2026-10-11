---
name: mcp-compat
description: MCP compatibility of the AgentRQ connectors host: the MCP revisions served (2026-07-28 and 2025-11-25), how a request's revision is chosen, and pinned behaviors. Use when checking what MCP surface a connector can rely on.
---

# MCP compatibility

`connectors serve` serves the two latest MCP revisions, **2026-07-28** and
**2025-11-25**, on the same endpoint: `POST /mcp` (Streamable HTTP). No MCP
SDK is used. Both revisions are run through
[mcp-spec-test](https://www.npmjs.com/package/@hasmcp/mcp-spec-test) in CI
(`scripts/conformance.sh`), and both get the verdict "conformant on what could
be checked". The behaviour below is pinned by golden tests in
`crates/mcp/tests/protocol.rs` and end-to-end tests in
`bin/connectors/tests/e2e.rs`.

## Which revision serves a request

Each POST is routed on its own, because neither revision keeps sessions here:

1. `initialize` is **2025-11-25**. A client asking for 2025-11-25, 2025-06-18
   or 2025-03-26 is answered with 2025-11-25. No `Mcp-Session-Id` is issued.
2. A request with `_meta["io.modelcontextprotocol/protocolVersion"]` is
   **2026-07-28**. Only `2026-07-28` is accepted there; any other value gets
   HTTP 400 `-32022` with `data: {supported: ["2026-07-28", "2025-11-25"], requested}`.
3. A request with an `MCP-Protocol-Version: 2025-*` header, or with no version
   at all, is **2025-11-25**. The one exception is a version-less
   `server/discover`: that method exists only in 2026-07-28, so it is served
   there.

On 2026-07-28 the checks run in this order:

1. `_meta` must be present: a `MCP-Protocol-Version: 2026-07-28` header with
   no `_meta` gets 400 `-32602`.
2. The version must be supported (400 `-32022`).
3. The headers must agree with the body (400 `-32020`):
   `MCP-Protocol-Version` must equal the `_meta` version, `Mcp-Method` the
   method, and `Mcp-Name` the tool name (`tools/call`), the URI
   (`resources/read`) or the prompt name (`prompts/get`). `Mcp-Name` may be
   plain or `=?base64?…?=`.

## Behaviour by method

| Method | 2026-07-28 | 2025-11-25 |
|--------|------------|------------|
| `server/discover` | `supportedVersions`, `capabilities: {tools: {}, resources: {}, prompts: {}, extensions: {"io.modelcontextprotocol/skills": {directoryRead: true}}}`, `instructions`, `resultType`, `ttlMs`, `cacheScope: "private"`, `_meta.serverInfo` | `-32601` in a JSON-RPC body (HTTP 200), never a bare 404 |
| `initialize` | n/a: routed to 2025-11-25 | `protocolVersion: "2025-11-25"`, `capabilities: {tools: {}, resources: {}, prompts: {}}`, `serverInfo`, `instructions` |
| `notifications/initialized` | n/a | 202 |
| `ping` | 404 `-32601` | `{}` |
| `tools/list` | `[callTool]` plus `resultType`, `ttlMs` (1 h), `cacheScope: "private"`, `_meta.serverInfo` | `[callTool]` |
| `tools/call` | `resultType: "complete"`, or `"input_required"` for auth | plain `CallToolResult` |
| `resources/list`, `resources/templates/list`, `prompts/list` | Every connector's items, namespaced, 100 per page with `nextCursor`, plus `resultType`, `ttlMs` (5 min), `cacheScope: "private"`, `_meta.serverInfo` | The same items and `nextCursor` |
| `resources/read` | `contents` with `resultType`, `ttlMs: 0`, `cacheScope: "private"`, or `"input_required"` for auth | plain `ReadResourceResult`, or `-32042` for auth |
| `prompts/get` | `messages` with `resultType: "complete"`, or `"input_required"` for auth | plain `GetPromptResult`, or `-32042` for auth |
| `skills/list` | Every connector's skills (SEP-2640), 100 per page with `nextCursor`, plus `resultType`, `ttlMs` (5 min), `cacheScope: "private"`, `_meta.serverInfo` | `-32601` (HTTP 200) |
| `skills/get` | `skill` with `resultType`, `ttlMs` (5 min), `cacheScope: "private"`; an unknown skill is `-32602` | `-32601` (HTTP 200) |
| `resources/directory/read` | The children of a skill's folder, 100 per page with `nextCursor`, plus `resultType`; an unknown folder is `-32602` | `-32601` (HTTP 200) |
| `resources/read` of a `skill://` file | `contents` with `resultType`, `ttlMs: 0`, `cacheScope: "private"` | `-32602`: not a resource of this revision |
| `notifications/cancelled` | 202 | 202, and the call it names is answered with `-32800` |
| Other notifications | 202 | 202 |
| Unknown method | 404 `-32601` | `-32601` (HTTP 200) |
| A JSON-RPC response sent by the client | 400 | 202, ignored: the server never sends requests |

The 2026-07-28 fields (`resultType`, `ttlMs`, `cacheScope`, `_meta`) never
appear on a 2025-11-25 response. A test checks every legacy response for
them.

## Both revisions

- **Methods.** Only POST: `GET` and `DELETE` on `/mcp` get 405 with
  `Allow: POST`, so there is no GET stream. `GET /healthz` answers `ok`.
- **Checks on every POST, in order:**
  1. `Origin`: a request from an origin that is not allowed gets 403. Loopback
     and `--domain` are allowed, plus each `--allow-origin`. A request with no
     `Origin` passes.
  2. The token: 401 with `WWW-Authenticate: Bearer realm="connectors"`, and
     `error="invalid_token"` for a wrong token.
  3. `Content-Type` other than `application/json`: 415.
  4. `Accept` that allows neither `application/json` nor
     `text/event-stream`: 406. No `Accept` header means JSON.
  5. A body over 4 MiB: 413.

  These refusals carry a JSON-RPC `-32600` body with no `id`.
- **JSON-RPC errors.** A parse error is `-32700` and an invalid request is
  `-32600`, both HTTP 400. Batches are refused with `-32600`. An error whose
  request id is unknown leaves `id` out rather than sending `null`.
- **HTTP status of other JSON-RPC errors.** 2026-07-28 uses 400 for the
  `_meta`, version and header errors, and 404 for unknown methods. Everything
  else on both revisions uses 200, so a 2025-11-25 client never reads a 404
  as "session expired".
- **`MCP-Protocol-Version`.** Every response echoes its revision's version in
  this header, except the 400s above.
- **Streaming.** A `tools/call` with `_meta.progressToken`, from a client that
  accepts `text/event-stream`, becomes an SSE stream if it runs longer than
  10 s. The stream sends a `notifications/progress` event every 10 s
  ("Running", progress 1, 2, …), then the response. A client that accepts
  only SSE gets every answer as a single event. SSE responses carry
  `X-Accel-Buffering: no`.
- **Timeouts.** A call is limited to 120 s. After that it is answered with
  `-32603` (HTTP 504, or the last event of a stream).
- **Tools.** `tools/list` always returns exactly one tool, `callTool`, and
  the list never changes, so no `listChanged` notifications are sent. Calling any other tool
  name gets `-32602`. Bad `callTool` arguments or a command that does not parse are
  tool results with `isError: true`, so the model can see what to fix.
- **Resources and prompts.** Every connector's resources and prompts are
  listed in one namespace: `connectors://<c>[@<profile>]/<rest>` for
  resources (`gmail://labels` is `connectors://gmail/labels`; a URI with
  another scheme is kept whole after the connector) and `<c>[@<profile>].<name>`
  for prompts. The lists are read from the manifests (no connector runs),
  hold only what the caller's profile exposes, and change only when
  connectors are installed or removed, so no `listChanged` or
  `subscribe` is offered. A cursor the server did not hand out is `-32602`.
  `resources/read` and `prompts/get` run `<c>.resources read` and
  `<c>.prompts get`; a failure is `-32602` with the command's message, and a
  missing sign-in is asked for as on `tools/call` (or is the `-32602`
  message without URL elicitation). See README, "Resources and prompts".
- **Skills (2026-07-28 only).** The `io.modelcontextprotocol/skills`
  extension ([SEP-2640](https://modelcontextprotocol.io/seps/2640-skills-extension)):
  connectors that serve Agent Skills are listed by `skills/list`, each under
  its connector's name, `skill://<c>/<skill-path>/SKILL.md`. Every file is a
  plain resource, read with `resources/read`, and listed with its `sha256:`
  digest and size; `resources/directory/read` lists a skill's folders. The
  connectors are asked when the list is (skills may be fetched by the
  connector on demand); one that fails is left out. Skills have no
  `callTool` form. 2025-11-25 has no `extensions` capability, so it serves
  none of this. See README, "Skills".
- **Output format.** `callTool` takes `"format": "json"|"toon"`; without it,
  the profile's choice applies. With TOON, JSON in text blocks and in JSON
  embedded resources becomes TOON (`mimeType: text/toon`), and the result has
  no `structuredContent`. That is allowed because `callTool` declares no
  `outputSchema`. Images, audio and base64 blobs are left alone. See README,
  "Output formats".

## Sign-in from MCP

When a command needs a credential the connector does not have yet (for the
command's profile; the link goes to `/connect/<c>@<profile>` for a profile
other than `default`):

| Client | Answer |
|--------|--------|
| 2026-07-28, declares `clientCapabilities.elicitation.url` | `resultType: "input_required"` with `inputRequests.auth`, a URL elicitation to the server's `/connect/<c>` page, plus a sealed `requestState`. The client retries with `inputResponses` and the same `requestState`; the server re-runs the command and answers again until sign-in is done. |
| 2025-11-25, declared URL elicitation in its last `initialize` | Error `-32042` (URL elicitation required) with `data.elicitations[]` pointing to `/connect/<c>`. The message holds the link as well. |
| Either revision, no URL elicitation | A text tool error with the link. |

The `requestState` is XChaCha20-Poly1305 over the expiry, bound to the tool
name, the arguments and the caller. It lasts 10 minutes. A tampered or
expired state gets `-32602`, and a declined or cancelled elicitation becomes a
tool error. When a connector can sign in by itself (client credentials or a
refresh token), the server does so and the call simply succeeds.

## OAuth for MCP clients

`/mcp` is an OAuth 2.1 protected resource (docs/PLAN.md §5.6):

- A request without a token gets `401` with
  `WWW-Authenticate: Bearer realm="connectors", resource_metadata="<base>/.well-known/oauth-protected-resource", scope="mcp"`.
  The same metadata is at `/.well-known/oauth-protected-resource/mcp`.
- There are two scopes. `mcp` opens `/mcp`. `api` opens the management API
  `/api/v1` as the profile (see below). `/mcp`'s challenges carry
  `scope="mcp"` and its metadata lists `["mcp"]`; the authorization server
  lists `["mcp", "api"]`. `/oauth2/authorize` takes `mcp`, `api` or both;
  no `scope` means `mcp`, and any other name gets `invalid_scope`. The
  consent page says what is asked, the grant keeps it, and token responses
  return it (`"scope": "mcp api"`). A refresh keeps its grant's scopes,
  whatever `scope` it sends. Which profile the token opens is picked on the
  consent page and carried in its `sub`, not in the scope.
- A valid token without the scope an endpoint needs gets `403` with
  `WWW-Authenticate: Bearer realm="connectors", error="insufficient_scope", error_description="…", resource_metadata="…", scope="<needed>"`.
- The authorization server is the host itself: `/.well-known/oauth-authorization-server`,
  with `/oauth2/register`, `/oauth2/authorize` and `/oauth2/token`. It
  supports the `code` response type, the `authorization_code` and
  `refresh_token` grants, public clients (`none`), PKCE `S256` only, and the
  `iss` response parameter. A `resource` parameter, if sent, must be
  `<base>/mcp` or `<base>/api/v1`.
- The consent page lists the user's profiles; the token is bound to the one
  picked. Its `sub` is `<org id>:<user id>@<profile>`, for example
  `00000000001:00000000001@work`.
  Its `scope` claim holds the scopes (`"mcp api"`), and `aud` the URLs they
  open (`<base>/mcp`, `<base>/api/v1`; an array when there are both).
- Through that token, `connectors` and `searchTools` list only the profile's
  connectors, and every command runs with the profile's credential. A
  command that names another profile (`gmail@home`) is refused.

### The `api` scope

`/api/v1` takes the owner's token, or an access token with `api`, which acts
as its profile. Without a token it answers `401` with
`resource_metadata="<base>/.well-known/oauth-protected-resource/api/v1", scope="api"`;
that metadata names the resource `<base>/api/v1` and lists `["api"]`.

An `api` token may:

- read `GET /info`, `GET /connectors` (the profile's connectors only, as the
  profile), and one of them with its tools, resources, prompts, skills and
  files;
- sign one of them in and out, and delete its files;
- read `GET /profiles` (its own profile only);
- opt its profile in or out of connectors and bundles, pick what they
  expose, set its format, and revoke its apps.

A connector address without a profile (`gmail`) means the token's profile.
One naming another profile, or a connector the profile does not opt into,
gets `403`. So does every other route: installing and removing, remote
servers, the catalog, inputs, refreshing, file quotas, bundle changes,
creating and deleting profiles, GitHub, and the setup.

The clone routes `/api/v1/upstream/*` take the owner's token or an `api`
token; an `mcp` token gets `403 insufficient_scope`. An `api` token clones
its profile only, and its secrets are the profile's own credentials. The
OAuth clients and inputs every profile shares go to the owner's token alone.
`connectors upstream login` (proxy and clone) asks for `mcp api`.

## Remote MCP servers

A remote connector (see the `connector-manifest` skill) makes the host an MCP
**client** of another server. Its requests go through the same egress as a
component's, so they carry the stored credential of the call's profile,
refreshed when it expires and once more after a 401.

- **Revision.** The client starts with the 2025-11-25 handshake: `initialize`,
  then `notifications/initialized`, keeping the `Mcp-Session-Id` the server
  gives and the version it chose (2025-06-18 and 2025-03-26 servers work
  too). A 404 to a request in a session starts a new one, once. A server that
  refuses `initialize` is spoken to on stateless 2026-07-28: `server/discover`,
  then every request with `_meta` protocol version and client capabilities and
  the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` headers.
- **Responses** may be JSON or an event stream. In a stream, notifications are
  skipped and the response to the request is used.
- **Listing** follows `nextCursor` (up to 100 pages) for `tools/list`,
  `resources/list`, `resources/templates/list` and `prompts/list`, only for
  the capabilities the server declares. A server that answers a list with
  `-32601` lists nothing there.
- **Skills.** Listing also asks `server/discover`, statelessly, even of a
  server spoken to on 2025: one that declares the skills extension is
  installed with `"skills": true`, and its skills are served under the
  connector's name. `skills/list`, `skills/get`, `resources/directory/read`
  and reads of `skill://` files are sent on 2026-07-28. Skills under another
  scheme than `skill://` are not served.
- **Calls.** `<c>.tools <tool>` sends `tools/call` with the server's name of
  the tool and the body as `arguments`; `<c>.resources read` sends
  `resources/read`, and `<c>.prompts get` sends `prompts/get`. A JSON-RPC
  error `-32602` is reported as invalid input, `-32601` and `-32002` as not
  found, and others as upstream errors. A 401 says to sign in again.
- **Authorization** discovery is described in
  the `connector-manifest` skill. Sign-ins send the RFC 8707
  `resource`, and a client is registered (RFC 7591) when the server allows it
  and none is configured. The registered client remembers its redirect URIs;
  the CLI listens on the loopback port it was registered with when it is
  free, and a new loopback port replaces the old one in the next
  registration.

What the client does not do:

| Not supported | Effect |
|---------------|--------|
| Server-to-client requests: sampling, elicitation, roots, ping | They are not answered. A call that waits for one runs into its time limit. |
| 2026-07-28 `input_required` results | The call fails with "the server asked for input". |
| `notifications/*/list_changed` and resource subscriptions | The listing changes only with `connectors refresh <c>`, a sign-in, or the Refresh button. |
| Client ID Metadata Documents, and the 2025-03-26 default endpoints (`/authorize`, `/token`) without metadata | A server that needs them is installed with a bearer token scheme. |
| stdio servers | Only Streamable HTTP URLs are taken. |

## Known deviations

| Deviation | Why |
|-----------|-----|
| **Opt-in `?token=` auth** (`--allow-query-token`). The 2026-07-28 spec says tokens must never be sent in a query string. | Some MCP clients cannot set headers. The mode is off by default and must be turned on explicitly, and the header token keeps priority. Tokens in URLs can end up in proxy and server logs, so use it only over HTTPS and rotate the token. mcp-spec-test's `--auth-mode query` covers it in CI. |
| **The owner's static bearer token is accepted next to OAuth 2.1.** | It is how the CLI, scripts and clients without OAuth reach every profile (D3). OAuth apps get one profile each; see "OAuth for MCP clients". |
| **No Client ID Metadata Documents for clients**, only dynamic registration (RFC 7591), and no `client_credentials`. | Every MCP client that does OAuth supports registration; CIMD can be added later. |
| **A `_meta` version of `2025-11-25` gets `-32022`.** 2026-07-28 requests must carry `2026-07-28`; older clients are sent to `initialize` by the `supported` list. | One clear path per revision. As a result, mcp-spec-test's 2026-07-28 check "older client receives no newer fields" is reported as not verified; e2e.rs checks it on the legacy route instead. |
| **A version-less `server/discover` is served on 2026-07-28**, although version-less requests otherwise default to 2025-11-25. | mcp-spec-test requires "a request with no version at all is served on the default". With a `2025-*` header it is still `-32601`. |
| **No sessions on 2025-11-25** (no `Mcp-Session-Id`, no GET stream). | Allowed by the spec, and nothing here needs server-to-client requests (D9). Because of this, "declared URL elicitation" means the capabilities from the latest `initialize` of the same caller, and up to 1024 callers are remembered. With one shared token, every client is the same caller. |
| **An unknown `cursor` on `tools/list` is answered, not refused with `-32602`.** | There is only one page of tools. mcp-spec-test reports it as a SHOULD that is not met. |
| **Cancels are matched by caller and request id.** A `notifications/cancelled` stops the in-flight call with that `requestId` from the same caller. | There are no sessions (D9), so the caller is the principal. With one shared token, every client is the same caller: two clients that both have a call with the same id in flight can cancel each other's, and the one registered last is the one a cancel reaches. Give each client its own server or make ids unique per client. |
| **Progress events are heartbeats.** | Connectors cannot report progress yet. The events keep proxies and clients from timing out long calls. |

## Running the conformance suite

```sh
cargo build --release -p connectors
cargo build --release --target wasm32-wasip2 -p connectors-fixture-echo -p connectors-example-httpbin
scripts/conformance.sh
```

The script serves echo and httpbin, then runs mcp-spec-test on both revisions
three times: without auth, with `-t <token>`, and with `--auth-mode query`.
Each run must exit 0, and its report must say "conformant on what could be
checked" or better. PR CI pins the spec-test version (`SPEC_TEST_VERSION` in
`.github/workflows/ci.yml`). A nightly job runs `@latest` and opens or
comments on a `spec-test-drift` issue when the result changes.
