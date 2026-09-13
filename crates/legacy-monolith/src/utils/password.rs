use crate::utils::error::AppError;

pub fn hash_password(plain: &str) -> Result<String, AppError> {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
}

pub fn verify_password(plain: &str, hashed: &str) -> Result<bool, AppError> {
    bcrypt::verify(plain, hashed).map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
}
