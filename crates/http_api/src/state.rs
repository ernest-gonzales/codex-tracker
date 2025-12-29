use rand::RngCore;

use app_api::AppContext;

#[derive(Clone)]
pub struct HttpState {
    pub context: AppContext,
    pub csrf_token: String,
}

impl HttpState {
    pub fn new(context: AppContext, csrf_token: String) -> Self {
        Self {
            context,
            csrf_token,
        }
    }
}

pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}
