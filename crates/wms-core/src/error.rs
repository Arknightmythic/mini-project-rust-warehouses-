#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Unavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => AppError::NotFound("resource not found".to_string()),
            // Postgres SQLSTATE codes. These are client mistakes, not server
            // faults, so they must not fall through to a 500.
            sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                // unique_violation
                Some("23505") => AppError::Conflict("resource already exists".to_string()),
                // foreign_key_violation
                Some("23503") => {
                    AppError::Validation("referenced resource does not exist".to_string())
                }
                // check_violation
                Some("23514") => {
                    AppError::Validation("value violates a database constraint".to_string())
                }
                // not_null_violation
                Some("23502") => AppError::Validation("a required field is missing".to_string()),
                _ => AppError::Internal(anyhow::anyhow!(err)),
            },
            _ => AppError::Internal(anyhow::anyhow!(err)),
        }
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::Unavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            AppError::Internal(err) => {
                tracing::error!(error = ?err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, axum::Json(serde_json::json!({ "message": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
