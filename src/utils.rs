pub fn are_mime_compatible(accepted: &mime_guess::Mime, actual: &mime_guess::Mime) -> bool {
    (accepted.type_() == mime_guess::mime::STAR || accepted.type_() == actual.type_())
        && (accepted.subtype() == mime_guess::mime::STAR
            || (accepted.subtype() == actual.subtype()))
}
