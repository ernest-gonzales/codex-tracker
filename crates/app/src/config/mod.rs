use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RangeParams {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}
