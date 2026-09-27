#[derive(Clone, Debug, Default)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub email: Option<String>,
}

impl UserContext {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
}
