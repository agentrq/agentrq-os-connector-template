// Connector lint: enforces the catalogue listing rules on this repository.
//
// A connector is listed only if its manifest.json carries every listing field:
// manifestVersion 1, mcpVersion, a valid connector name, version, a non-empty
// description, a non-empty icons array, and a non-empty instructions string.
// Manifest keys are written in alphanumeric order, at every level of the
// document. A repository publishing several connectors lists them in
// connectors-manifest.json instead, in alphanumeric order by manifest path;
// each listed manifest is checked the same way.
// Dependency-free: node only, no npm install.
//
// Usage: node scripts/lint.mjs   (run from the repository root)

import { readFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

const CONNECTOR_ID_RE = /^[a-z][a-z0-9-]{0,62}$/
const MANIFEST_FILE = 'manifest.json'
const INDEX_FILE = 'connectors-manifest.json'

const errors = []
const fail = (where, reason) => errors.push(`${where}: ${reason}`)

const isNonEmptyString = (value) => typeof value === 'string' && value.trim() !== ''
const isPlainObject = (value) => Boolean(value) && typeof value === 'object' && !Array.isArray(value)

function readJson(path) {
  let text
  try {
    text = readFileSync(path, 'utf8')
  } catch {
    return { ok: false, reason: 'file not found' }
  }
  try {
    return { ok: true, doc: JSON.parse(text) }
  } catch (error) {
    return { ok: false, reason: `not valid JSON: ${error.message}` }
  }
}

/** An icon src is an https: URL or a data: URI. */
function isValidIconSrc(src) {
  if (!isNonEmptyString(src)) return false
  if (src.startsWith('data:')) return true
  try {
    return new URL(src).protocol === 'https:'
  } catch {
    return false
  }
}

function checkIcons(doc, where) {
  if (!Array.isArray(doc.icons) || doc.icons.length === 0) {
    fail(where, '"icons" is required: a non-empty array, so the connector lists with an icon')
    return
  }
  doc.icons.forEach((icon, i) => {
    const at = `${where}: "icons"[${i}]`
    if (!isPlainObject(icon)) {
      fail(at, 'must be an object')
      return
    }
    if (!isValidIconSrc(icon.src)) {
      fail(at, '"src" must be an https: URL or a data: URI')
    }
    if (!isNonEmptyString(icon.mimeType)) {
      fail(at, '"mimeType" is required, e.g. "image/svg+xml"')
    }
    if (!Array.isArray(icon.sizes) || icon.sizes.length === 0 || !icon.sizes.every(isNonEmptyString)) {
      fail(at, '"sizes" must be a non-empty array of strings, e.g. ["any"]')
    }
  })
}

/** Every object's keys must be in alphanumeric order (code-point comparison). */
function checkKeyOrder(value, where) {
  if (Array.isArray(value)) {
    value.forEach((item, i) => checkKeyOrder(item, `${where}[${i}]`))
    return
  }
  if (!isPlainObject(value)) return
  const keys = Object.keys(value)
  for (let i = 1; i < keys.length; i++) {
    if (keys[i - 1] > keys[i]) {
      fail(where, `keys must be in alphanumeric order: "${keys[i]}" should come before "${keys[i - 1]}"`)
      break
    }
  }
  for (const key of keys) checkKeyOrder(value[key], `${where}.${key}`)
}

function checkManifest(doc, where) {
  if (!isPlainObject(doc)) {
    fail(where, 'must be a JSON object')
    return
  }
  if (doc.manifestVersion !== 1) {
    fail(where, `"manifestVersion" must be 1, not ${JSON.stringify(doc.manifestVersion ?? null)}`)
  }
  if (!isNonEmptyString(doc.mcpVersion)) {
    fail(where, '"mcpVersion" is required — the MCP revision this was written against, e.g. "2026-07-28"')
  }
  const name = typeof doc.name === 'string' ? doc.name : ''
  if (!CONNECTOR_ID_RE.test(name)) {
    fail(
      where,
      `"name" ${JSON.stringify(name || '(none)')} is not a connector id: a lowercase letter, then up to 62 lowercase letters, digits or '-'`,
    )
  }
  if (!isNonEmptyString(doc.version)) {
    fail(where, '"version" is required, e.g. "0.1.0"')
  }
  if (!isNonEmptyString(doc.description)) {
    fail(where, '"description" is required: what the connector does, in one or two sentences')
  }
  checkIcons(doc, where)
  if (!isNonEmptyString(doc.instructions)) {
    fail(where, '"instructions" is required: guidance for the model using this connector')
  }
}

/** A repository-relative manifest path that cannot escape the repository. */
function checkManifestPath(value, where) {
  if (!isNonEmptyString(value) || !/^[A-Za-z0-9\-_./]+$/.test(value) || !value.endsWith('.json')) {
    fail(where, `"manifest" ${JSON.stringify(value ?? '(none)')} is not a manifest path: it must end in ".json"`)
    return null
  }
  const segments = value.split('/')
  if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) {
    fail(where, `"manifest" "${value}" must not contain empty, "." or ".." segments`)
    return null
  }
  return value
}

function main() {
  const root = resolve(process.argv[2] ?? '.')

  const indexPath = join(root, INDEX_FILE)
  let manifestPaths = [MANIFEST_FILE]
  if (existsSync(indexPath)) {
    const index = readJson(indexPath)
    if (!index.ok) {
      fail(INDEX_FILE, index.reason)
    } else if (!isPlainObject(index.doc)) {
      fail(INDEX_FILE, 'must be a JSON object')
    } else {
      if (index.doc.manifestVersion !== 1) {
        fail(INDEX_FILE, `"manifestVersion" must be 1, not ${JSON.stringify(index.doc.manifestVersion ?? null)}`)
      }
      if (!Array.isArray(index.doc.connectors) || index.doc.connectors.length === 0) {
        fail(INDEX_FILE, '"connectors" must list at least one connector')
      } else {
        const paths = []
        index.doc.connectors.forEach((entry, i) => {
          const at = `${INDEX_FILE}: "connectors"[${i}]`
          if (!isPlainObject(entry)) {
            fail(at, 'must be an object with a "manifest" path')
            return
          }
          const path = checkManifestPath(entry.manifest, at)
          if (path === null) return
          if (paths.includes(path)) {
            fail(at, `lists "${path}" twice`)
            return
          }
          paths.push(path)
        })
        const ordered = [...paths].sort()
        for (let i = 0; i < paths.length; i++) {
          if (paths[i] !== ordered[i]) {
            fail(
              INDEX_FILE,
              `"connectors" must list manifests in alphanumeric order: "${ordered[i]}" sorts before "${paths[i]}"`,
            )
            break
          }
        }
        if (paths.length > 0) manifestPaths = paths
      }
    }
  }

  for (const path of manifestPaths) {
    const full = join(root, path)
    const read = readJson(full)
    if (!read.ok) {
      fail(path, read.reason)
      continue
    }
    checkManifest(read.doc, path)
    checkKeyOrder(read.doc, path)
  }

  if (errors.length > 0) {
    console.error(`connector lint failed: ${errors.length} problem${errors.length === 1 ? '' : 's'}\n`)
    for (const error of errors) console.error(`  - ${error}`)
    process.exit(1)
  }
  console.log(`connector lint passed: ${manifestPaths.join(', ')} meet the listing rules.`)
}

main()
