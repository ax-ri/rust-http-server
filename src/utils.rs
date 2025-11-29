use mime_guess::{Mime, mime};

pub(crate) fn are_mime_compatible(accepted: &Mime, actual: &Mime) -> bool {
    (accepted.type_() == mime::STAR || accepted.type_() == actual.type_())
        && (accepted.subtype() == mime::STAR || (accepted.subtype() == actual.subtype()))
}
