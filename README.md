# AgentRQ connector template

A starter repository for publishing [AgentRQ](https://agentrq.com) connectors.
One repository can publish several connectors — like
[agentrq-os-connectors](https://github.com/agentrq/agentrq-os-connectors) does:
each connector lives in its own folder with a `manifest.json`, an `icon.svg`
and its Rust sources, and a root `connectors-manifest.json` lists them all.
GitHub Actions workflows enforce the listing rules and the Rust quality bar on
every push and pull request.

```
.
├── connectors-manifest.json   # the index: every connector this repo publishes
├── my-connector/
│   ├── manifest.json          # the connector's listing (icons + instructions required)
│   ├── icon.svg               # the connector's icon
│   ├── Cargo.toml
│   └── src/lib.rs             # example implementation + tests
├── scripts/lint.mjs           # the listing-rules linter (dependency-free)
├── .claude/skills/              # skills, auto-discovered by Claude Code:
│   ├── build-connector/         # scaffold, build, test and publish a connector
│   ├── connector-authoring/     # end-to-end authoring guide
│   ├── connector-manifest/      # complete manifest.json reference
│   └── mcp-compat/              # MCP revisions and pinned host behaviors
└── .github/workflows/
    ├── lint.yml               # connector lint, on push and PR
    └── test.yml               # fmt, clippy, tests, 100% coverage
```

![icon](my-connector/icon.svg)

## Use this template

1. **Use this template** (the green button on GitHub) to create your connector
   repository.
2. In `my-connector/manifest.json`: rename `my-connector`, write a real
   `description` and `instructions`, point `icons[0].src` at your icon, and
   describe your tools. Replace `<owner>/<repo>` in the icon URL with your
   repository.
3. Replace `my-connector/icon.svg` with your service's icon and
   `my-connector/src/lib.rs` with your connector's implementation.
4. Push. The `connector lint` and `tests` workflows must both be green.
5. Add the [`agentrq-connector`](https://github.com/topics/agentrq-connector)
   topic to the repository so the public catalogue discovers it.

## Building the connector

This template bundles four skills from `agentrq/connectors`, picked up
automatically by Claude Code:

- **`build-connector`** — plan the tool surface, scaffold the crate from the
  `minimal`, `api-key` or `oauth2` templates, implement tools against
  `connectors-sdk`, and test on the host before building the `.wasm`.
- **`connector-authoring`** — the end-to-end authoring guide: scaffolding,
  tools, auth, building, installing and testing.
- **`connector-manifest`** — the complete `manifest.json` field reference,
  including the listing requirements (`icons` and `instructions` are
  mandatory to be listed).
- **`mcp-compat`** — which MCP revisions the host serves and how a request's
  revision is chosen.

The skills' checklists encode the same quality bar the workflows enforce:
`icons` and `instructions` in every manifest, `cargo fmt` clean, clippy with
zero warnings, and 100% line and function coverage.

## Adding another connector

1. Copy `my-connector/` to a new folder, e.g. `my-other-connector/`, and edit
   its `manifest.json` (new `name`, `description`, `instructions`, `icons`).
2. Add the folder to the workspace `members` in the root `Cargo.toml`.
3. Add `{ "manifest": "my-other-connector/manifest.json" }` to
   `connectors-manifest.json`.

The lint script validates the index itself and then applies the listing rules
to **every** manifest it points at, reporting per-connector pass/fail with the
connector name and the exact rule violated.

## `manifest.json` field reference

| Field | Required to be listed | Notes |
|---|---|---|
| `manifestVersion` | yes | Always `1`. |
| `mcpVersion` | yes | The MCP revision this was written against, e.g. `"2026-07-28"`. |
| `name` | yes | The connector id: `^[a-z][a-z0-9-]{0,62}$` — a lowercase letter, then lowercase letters, digits or `-`. It becomes the command prefix (`my-connector.tools`). |
| `version` | yes | e.g. `"0.1.0"`. |
| `title` | no | Display name; the name stands in when absent. |
| `description` | yes | One or two sentences: what the connector does. This is what people read in the catalogue. |
| `websiteUrl` | no | The upstream service's website. |
| `icons` | yes | Non-empty array. Each entry needs `src` (an `https:` URL or `data:` URI), `mimeType` (e.g. `"image/svg+xml"`), and `sizes` (e.g. `["any"]`). A connector with no icon is not listed. |
| `instructions` | yes | Guidance for the model using the connector: when to call it, how its tools fit together, and any conventions it follows. Two to four sentences are usually enough. A connector without instructions is not listed. |
| `tools` | no | The tools the connector exports (`name`, `title`, `description`, `inputSchema`). |
| `auth` | no | How the connector authenticates to its upstream service. |
| `inputs` | no | Values the user supplies at install time. |
| `permissions` | no | What the connector may reach (e.g. `http.allow`). |
| `limits` | no | Resource limits the connector asks for. |
| `$schema` | no | `https://connectors.agentrq.com/schemas/manifest/v1.json`, for editors. |

`icons` and `instructions` are mandatory to be listed: the `connector lint`
workflow fails the build when either is missing or empty.

## The index

`connectors-manifest.json` at the root lists every connector the repository
publishes:

```json
{
  "$schema": "https://connectors.agentrq.com/schemas/connectors-manifest/v1.json",
  "manifestVersion": 1,
  "connectors": [
    { "manifest": "my-connector/manifest.json" },
    { "manifest": "my-other-connector/manifest.json" }
  ]
}
```

(A repository publishing a single connector may instead keep its
`manifest.json` at the root with no index; the lint script handles both.)
When the index exists, the lint script validates the index itself (valid JSON,
`manifestVersion` 1, sane and unique `manifest` paths) and then applies the
listing rules to **every** manifest it points at, reporting per-connector
pass/fail with the connector name and the exact rule violated.

## Quality bar

Both workflows must be green before a connector is published:

- **Manifest lint** (`connector lint`): `node scripts/lint.mjs` checks every
  published manifest against the listing rules. Run it locally before pushing.
- **Rust gates** (`tests`): `cargo fmt --all -- --check` must be clean,
  `cargo clippy --all-targets -- -D warnings` must report zero warnings,
  `cargo test` must pass, and `cargo llvm-cov --fail-under-lines 100
  --fail-under-functions 100` must hold — 100% line and function coverage is
  mandatory, so every function and branch needs a test.

## License

Apache-2.0. See [LICENSE](LICENSE).
