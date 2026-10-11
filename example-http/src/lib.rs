//! Example: outbound HTTP with an injected client.
//!
//! The API key lives in the `api_key` input (`secret: true`): the host stores
//! it and injects it as the `X-Api-Key` header after the request leaves the
//! connector. The code below never touches the key — it builds the request
//! URL and interprets the response. The `HttpClient` trait is injected so
//! tests can stub the network.

/// An outbound HTTP client. The real one lives on the host; tests stub it.
pub trait HttpClient {
    /// Performs a GET request, returning the status code and body.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be made.
    fn get(&mut self, url: &str) -> Result<(u16, String), String>;
}

/// Builds the URL for the `getStatus` tool.
#[must_use]
pub fn status_url(api_base: &str, code: u16) -> String {
    format!("{}/status/{code}", api_base.trim_end_matches('/'))
}

/// Asks the API for `code` and returns its response body.
///
/// # Errors
///
/// Returns an error when the request fails or the API answers with a
/// non-200 status.
pub fn get_status(
    client: &mut dyn HttpClient,
    api_base: &str,
    code: u16,
) -> Result<String, String> {
    let (status, body) = client.get(&status_url(api_base, code))?;
    if status == 200 {
        Ok(body)
    } else {
        Err(format!("unexpected status {status}: {body}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub {
        answer: Result<(u16, String), String>,
        seen: Vec<String>,
    }

    impl Stub {
        fn ok(body: &str) -> Self {
            Self {
                answer: Ok((200, body.to_string())),
                seen: Vec::new(),
            }
        }
    }

    impl HttpClient for Stub {
        fn get(&mut self, url: &str) -> Result<(u16, String), String> {
            self.seen.push(url.to_string());
            self.answer.clone()
        }
    }

    #[test]
    fn builds_the_status_url() {
        assert_eq!(
            status_url("https://api.example.com", 418),
            "https://api.example.com/status/418"
        );
    }

    #[test]
    fn trims_a_trailing_slash_from_the_base() {
        assert_eq!(
            status_url("https://api.example.com/", 200),
            "https://api.example.com/status/200"
        );
    }

    #[test]
    fn returns_the_body_on_200() {
        let mut client = Stub::ok("teapot");
        assert_eq!(
            get_status(&mut client, "https://api.example.com", 418),
            Ok("teapot".to_string())
        );
        assert_eq!(client.seen, ["https://api.example.com/status/418"]);
    }

    #[test]
    fn rejects_a_non_200_status() {
        let mut client = Stub {
            answer: Ok((500, "boom".to_string())),
            seen: Vec::new(),
        };
        assert_eq!(
            get_status(&mut client, "https://api.example.com", 200),
            Err("unexpected status 500: boom".to_string())
        );
    }

    #[test]
    fn propagates_a_request_failure() {
        let mut client = Stub {
            answer: Err("dns failed".to_string()),
            seen: Vec::new(),
        };
        assert_eq!(
            get_status(&mut client, "https://api.example.com", 200),
            Err("dns failed".to_string())
        );
    }
}
