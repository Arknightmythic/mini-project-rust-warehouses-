use tonic::{Code, Status};

use crate::error::AppError;

// Written once so every handler on both sides of a boundary is just `?`.
impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(msg) => Status::new(Code::NotFound, msg),
            AppError::Validation(msg) => Status::new(Code::InvalidArgument, msg),
            AppError::Conflict(msg) => Status::new(Code::AlreadyExists, msg),
            AppError::Unauthorized => Status::new(Code::Unauthenticated, "unauthorized"),
            AppError::Forbidden => Status::new(Code::PermissionDenied, "forbidden"),
            AppError::Unavailable(msg) => Status::new(Code::Unavailable, msg),
            AppError::Internal(err) => {
                // Scrub outbound too: a downstream's SQL error text must never
                // travel further up the chain toward a client.
                tracing::error!(error = ?err, "internal error crossing a service boundary");
                Status::new(Code::Internal, "internal server error")
            }
        }
    }
}

impl From<Status> for AppError {
    fn from(status: Status) -> Self {
        let message = status.message().to_string();

        match status.code() {
            Code::NotFound => AppError::NotFound(message),
            Code::InvalidArgument | Code::OutOfRange => AppError::Validation(message),
            Code::AlreadyExists | Code::FailedPrecondition | Code::Aborted => {
                AppError::Conflict(message)
            }
            Code::Unauthenticated => AppError::Unauthorized,
            Code::PermissionDenied => AppError::Forbidden,
            Code::Unavailable | Code::DeadlineExceeded => {
                AppError::Unavailable("upstream service unavailable".to_string())
            }
            _ => AppError::Internal(anyhow::anyhow!("upstream error: {message}")),
        }
    }
}
