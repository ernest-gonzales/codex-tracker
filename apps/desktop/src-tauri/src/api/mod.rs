pub(crate) mod handlers;

pub(crate) fn to_error(err: impl std::fmt::Display) -> String {
    err.to_string()
}
