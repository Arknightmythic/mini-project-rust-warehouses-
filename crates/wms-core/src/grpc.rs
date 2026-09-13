use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::{Code, Status};

use crate::error::AppError;
use crate::identity::Identity;

pub const USER_ID_KEY: &str = "x-user-id";
pub const USER_EMAIL_KEY: &str = "x-user-email";
pub const USER_ROLES_KEY: &str = "x-user-roles";

// The gateway CONSTRUCTS these from verified JWT claims. It never copies inbound
// HTTP headers through: a gateway that forwards raw headers can be bypassed
// completely by a client that simply sets X-User-Roles itself.
pub fn identity_to_metadata(identity: &Identity, metadata: &mut MetadataMap) {
    if let Ok(value) = MetadataValue::try_from(identity.user_id.to_string()) {
        metadata.insert(USER_ID_KEY, value);
    }
    if let Ok(value) = MetadataValue::try_from(identity.email.clone()) {
        metadata.insert(USER_EMAIL_KEY, value);
    }
    if let Ok(value) = MetadataValue::try_from(identity.roles.join(",")) {
        metadata.insert(USER_ROLES_KEY, value);
    }
}

// Downstream trusts this because only the gateway can reach it. That assumption
// is the trust boundary, and it is closed properly in a later phase.
pub fn identity_from_metadata(metadata: &MetadataMap) -> Result<Identity, AppError> {
    let get = |key: &str| -> Option<String> {
        metadata
            .get(key)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
    };

    let user_id = get(USER_ID_KEY)
        .and_then(|raw| raw.parse::<i64>().ok())
        .ok_or(AppError::Unauthorized)?;

    Ok(Identity {
        user_id,
        email: get(USER_EMAIL_KEY).unwrap_or_default(),
        roles: get(USER_ROLES_KEY)
            .map(|raw| {
                raw.split(',')
                    .filter(|part| !part.is_empty())
                    .map(|part| part.to_string())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

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
                // Scrub outbound too: a downstream SQL error must never travel
                // further up the chain toward a client.
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
