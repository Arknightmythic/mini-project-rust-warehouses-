use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: i64,
    pub email: String,
    pub roles: Vec<String>,
}

impl Identity {
    pub fn require_role(&self, role: &str) -> Result<(), AppError> {
        if self.roles.iter().any(|r| r == role) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn require_any_role(&self, roles: &[&str]) -> Result<(), AppError> {
        if roles.iter().any(|role| self.roles.iter().any(|r| r == role)) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}
