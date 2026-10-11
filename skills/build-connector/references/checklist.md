# Connector checklist

Go through every line before calling a connector done. The
`connector-reviewer` agent checks the same list.

## Manifest

- [ ] `name` is the connector id (`^[a-z][a-z0-9-]{0,62}$`); `version` is semver; `title` and `description` say what it is for.
- [ ] Tool names are camelCase, unique, at most 64 characters, and not `auth`, `help`, `search`, `searchTools`, `tools`, `resources` or `prompts`.
- [ ] Each tool `description` tells a model when to use it and what it returns, in one or two sentences.
- [ ] Every `inputSchema` has `"type": "object"` and `"additionalProperties": false`; required fields are in `required`; strings that must not be empty have `minLength: 1`; numbers have sensible `minimum`/`maximum`.
- [ ] `-q` / `-h` arguments are declared in `querySchema` / `headersSchema` (lowercase header names), or not used.
- [ ] `annotations` are honest: `readOnlyHint: true` only for reads; `destructiveHint: true` for anything that deletes or overwrites; `openWorldHint: true` for tools that call out.
- [ ] `permissions.http.allow` holds only the origins the code calls: https, no paths, no wildcards wider than needed. No `allowInsecure` unless the user asked for a plain-http service.
- [ ] `auth` matches how the API authenticates. OAuth scopes are the narrowest that cover the tools.
- [ ] Every `inputs` entry the code reads has a fallback for its `default`.

## Code

- [ ] No credential anywhere in code, manifest, tests or comments. The code never sets `authorization`, `cookie` or `proxy-*` headers (the host would drop them anyway).
- [ ] Every path segment and query value from input is percent-encoded; header values and RFC 822 headers lose `\r` and `\n`.
- [ ] Bodies are deserialised into typed structs with `request.json::<T>()?`, not poked at as `Value`.
- [ ] A non-2xx answer the model can act on is a `ToolResponse::error` with the status and body; a `ConnectorError` is only for calls that cannot produce a result.
- [ ] Structured results are JSON objects (wrap arrays).
- [ ] No reliance on state between calls; no `unwrap()`/`expect()` on input or responses.
- [ ] Nothing logs bodies, tokens or personal data.

## Tests and build

- [ ] `cargo test` passes with 100% line and function coverage (`cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100`): each tool's planned request, bad input, an unknown tool, and the response mapping for 2xx, an error status and a non-JSON body.
- [ ] `cargo build --release --target wasm32-wasip2` succeeds.
- [ ] `connectors install <wasm>` succeeds, and `connectors <name> tools <tool> help` reads well.
- [ ] If the user signed in: one real read-only call returns what it should.
