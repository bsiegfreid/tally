//! Content negotiation — which representation a request wants.
//! Canopy's `taproot::format`, cut to the two formats Tally serves.
//! The path extension is canonical (`/run.json`); `Accept:
//! application/json` is a courtesy fallback. HTML is the default.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Html,
    Json,
}

impl Format {
    /// Resolve the requested format from a request path and optional
    /// `Accept` header value.
    ///
    /// Parameters
    /// - `path`: the request path; a trailing `.json` wins outright.
    /// - `accept`: the request's `Accept` header value, if present.
    ///
    /// Returns the representation to serve — HTML unless something
    /// asked for JSON.
    pub fn negotiate(path: &str, accept: Option<&str>) -> Format {
        if path.ends_with(".json") {
            return Format::Json;
        }
        if accept.is_some_and(|a| a.contains("application/json")) {
            return Format::Json;
        }
        Format::Html
    }

    /// The MIME type for this representation, for the response
    /// `Content-Type`.
    pub fn content_type(self) -> &'static str {
        match self {
            Format::Html => "text/html; charset=utf-8",
            Format::Json => "application/json",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_wins_then_accept_then_html() {
        assert_eq!(Format::negotiate("/run.json", None), Format::Json);
        assert_eq!(
            Format::negotiate("/run", Some("application/json")),
            Format::Json
        );
        assert_eq!(Format::negotiate("/run", Some("text/html")), Format::Html);
        assert_eq!(Format::negotiate("/run", None), Format::Html);
    }
}
