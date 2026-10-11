//! Example: filesystem access with an injected filesystem.
//!
//! The manifest asks for `fs` `readwrite` access: the host then exposes a
//! private directory to the connector as `/data`. Tools take a note *name*,
//! never a raw path — the connector maps names into `/data/notes` itself, so
//! callers cannot escape the notes directory. The `Filesystem` trait is
//! injected so tests can use an in-memory stub.

/// The notes directory inside the connector's private `/data`.
pub const NOTES_DIR: &str = "/data/notes";

/// Host filesystem access. The real one lives on the host; tests stub it.
pub trait Filesystem {
    /// Reads the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read.
    fn read(&self, path: &str) -> Result<String, String>;

    /// Writes `content` to the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    fn write(&mut self, path: &str, content: &str) -> Result<(), String>;
}

/// Maps a note name to its path under [`NOTES_DIR`].
#[must_use]
pub fn note_path(name: &str) -> String {
    let slug: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{NOTES_DIR}/{slug}.md")
}

/// Reads the note `name`.
///
/// # Errors
///
/// Returns an error when the note cannot be read.
pub fn read_note(fs: &dyn Filesystem, name: &str) -> Result<String, String> {
    fs.read(&note_path(name))
}

/// Writes the note `name`.
///
/// # Errors
///
/// Returns an error when the note cannot be written.
pub fn write_note(fs: &mut dyn Filesystem, name: &str, content: &str) -> Result<(), String> {
    fs.write(&note_path(name), content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryFs {
        files: HashMap<String, String>,
    }

    impl Filesystem for MemoryFs {
        fn read(&self, path: &str) -> Result<String, String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| format!("no such file: {path}"))
        }

        fn write(&mut self, path: &str, content: &str) -> Result<(), String> {
            self.files.insert(path.to_string(), content.to_string());
            Ok(())
        }
    }

    #[test]
    fn maps_a_name_into_the_notes_dir() {
        assert_eq!(note_path("shopping"), "/data/notes/shopping.md");
    }

    #[test]
    fn sanitizes_note_names() {
        assert_eq!(note_path("  my note! "), "/data/notes/my-note-.md");
    }

    #[test]
    fn writes_and_reads_a_note() {
        let mut fs = MemoryFs::default();
        write_note(&mut fs, "todo", "buy milk").unwrap();
        assert_eq!(read_note(&fs, "todo"), Ok("buy milk".to_string()));
    }

    #[test]
    fn reports_a_missing_note() {
        let fs = MemoryFs::default();
        assert_eq!(
            read_note(&fs, "missing"),
            Err("no such file: /data/notes/missing.md".to_string())
        );
    }
}
