//! Example: MCP resources.
//!
//! The manifest declares a static resource (`resources`) and a parameterized
//! one (`resourceTemplates`). The `getResource` tool reads any declared URI,
//! so agents can fetch configuration or documents without a tool call per
//! item.

/// The static config resource, as JSON.
pub const CONFIG_JSON: &str = r#"{"version":"0.1.0","notesDir":"/data/notes"}"#;

/// The URIs of the static resources this connector exposes.
#[must_use]
pub fn static_resource_uris() -> &'static [&'static str] {
    &["example:///config"]
}

/// Expands the `note` resource template for `note_id`.
#[must_use]
pub fn note_uri(note_id: &str) -> String {
    format!("example:///notes/{note_id}")
}

/// Renders the body of the note resource `note_id`.
#[must_use]
pub fn note_body(note_id: &str) -> String {
    format!("# Note {note_id}\n\nThis is an example resource.\n")
}

/// Reads the resource at `uri`: a static resource or a `note` template
/// expansion.
///
/// # Errors
///
/// Returns an error naming the URI when it matches nothing declared.
pub fn get_resource(uri: &str) -> Result<String, String> {
    if static_resource_uris().contains(&uri) {
        return Ok(CONFIG_JSON.to_string());
    }
    if let Some(note_id) = uri.strip_prefix("example:///notes/") {
        if !note_id.is_empty() {
            return Ok(note_body(note_id));
        }
    }
    Err(format!("unknown resource: {uri}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_static_resource() {
        assert_eq!(static_resource_uris(), &["example:///config"]);
    }

    #[test]
    fn expands_the_note_template() {
        assert_eq!(note_uri("42"), "example:///notes/42");
    }

    #[test]
    fn reads_the_static_config() {
        assert_eq!(
            get_resource("example:///config"),
            Ok(CONFIG_JSON.to_string())
        );
    }

    #[test]
    fn reads_a_note_resource() {
        assert_eq!(
            get_resource("example:///notes/42"),
            Ok("# Note 42\n\nThis is an example resource.\n".to_string())
        );
    }

    #[test]
    fn rejects_an_unknown_resource() {
        assert_eq!(
            get_resource("example:///nope"),
            Err("unknown resource: example:///nope".to_string())
        );
    }

    #[test]
    fn rejects_an_empty_note_id() {
        assert_eq!(
            get_resource("example:///notes/"),
            Err("unknown resource: example:///notes/".to_string())
        );
    }
}
