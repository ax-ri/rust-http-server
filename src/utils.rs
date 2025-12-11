//! Utility functions used in various places around the project.

/// Check whether a Mime type is compatible with another, i.e. if the first is a superset of the second.
///
/// # Examples
///
/// ```
/// use rust_http_server::utils::are_mime_compatible;
/// assert!(are_mime_compatible(&mime_guess::mime::TEXT_STAR, &mime_guess::mime::TEXT_HTML));
/// assert!(!are_mime_compatible(&mime_guess::mime::TEXT_HTML, &mime_guess::mime::IMAGE_PNG));
/// assert!(are_mime_compatible(&mime_guess::mime::STAR_STAR, &mime_guess::mime::TEXT_HTML));
/// ```
pub fn are_mime_compatible(accepted: &mime_guess::Mime, actual: &mime_guess::Mime) -> bool {
    (accepted.type_() == mime_guess::mime::STAR || accepted.type_() == actual.type_())
        && (accepted.subtype() == mime_guess::mime::STAR
            || (accepted.subtype() == actual.subtype()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn are_mime_compatible_test() {
        assert!(are_mime_compatible(
            &mime_guess::mime::TEXT_STAR,
            &mime_guess::mime::TEXT_HTML
        ));
        assert!(are_mime_compatible(
            &mime_guess::mime::TEXT_STAR,
            &mime_guess::mime::TEXT_PLAIN
        ));
        assert!(are_mime_compatible(
            &mime_guess::mime::TEXT_STAR,
            &mime_guess::mime::TEXT_JAVASCRIPT
        ));
        assert!(are_mime_compatible(
            &mime_guess::mime::TEXT_HTML,
            &mime_guess::mime::TEXT_HTML
        ));

        assert!(!are_mime_compatible(
            &mime_guess::mime::TEXT_HTML,
            &mime_guess::mime::IMAGE_PNG
        ));
        assert!(!are_mime_compatible(
            &mime_guess::mime::APPLICATION_JSON,
            &mime_guess::mime::TEXT_HTML
        ));

        assert!(are_mime_compatible(
            &mime_guess::mime::STAR_STAR,
            &mime_guess::mime::TEXT_HTML
        ));
        assert!(are_mime_compatible(
            &mime_guess::mime::STAR_STAR,
            &mime_guess::mime::IMAGE_PNG
        ));
    }
}
