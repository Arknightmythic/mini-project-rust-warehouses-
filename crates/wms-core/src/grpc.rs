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

// --- W3C trace context over gRPC metadata ---------------------------------
//
// OpenTelemetry knows how to serialise a trace context into key/value pairs, but
// it has no idea what a tonic MetadataMap is. These two adapters are the bridge,
// written once here instead of in every service.

use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct MetadataInjector<'a>(pub &'a mut MetadataMap);

impl Injector for MetadataInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(value) = MetadataValue::try_from(value) {
                self.0.insert(name, value);
            }
        }
    }
}

pub struct MetadataExtractor<'a>(pub &'a MetadataMap);

impl Extractor for MetadataExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0
            .keys()
            .filter_map(|key| match key {
                tonic::metadata::KeyRef::Ascii(name) => Some(name.as_str()),
                tonic::metadata::KeyRef::Binary(_) => None,
            })
            .collect()
    }
}

// Client side. Unlike identity, trace context IS ambient: it lives in the current
// tracing span, so an interceptor can read it correctly and hiding it is right.
pub type TraceInterceptor = fn(tonic::Request<()>) -> Result<tonic::Request<()>, Status>;
pub type TracedChannel =
    tonic::service::interceptor::InterceptedService<tonic::transport::Channel, TraceInterceptor>;

pub fn inject_trace_context(
    mut request: tonic::Request<()>,
) -> Result<tonic::Request<()>, Status> {
    let context = tracing::Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut MetadataInjector(request.metadata_mut()))
    });

    Ok(request)
}

// Server side. The parent MUST be attached while the span is being CREATED:
// #[tracing::instrument] creates and enters its span before the body runs, so
// calling set_parent inside a handler fails with "span has already been started".
// A MakeSpan hook runs at exactly the right moment, and covers every handler at
// once instead of needing one line in each.
#[derive(Clone, Copy)]
pub struct GrpcMakeSpan;

impl<B> tower_http::trace::MakeSpan<B> for GrpcMakeSpan {
    fn make_span(&mut self, request: &http::Request<B>) -> tracing::Span {
        let span = tracing::info_span!("grpc", rpc = %request.uri().path());

        let parent = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(request.headers()))
        });

        // Safe here: the span exists but has not been entered yet.
        if let Err(err) = span.set_parent(parent) {
            tracing::debug!(error = %err, "could not attach the incoming trace context");
        }

        span
    }
}

pub struct HeaderExtractor<'a>(pub &'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect()
    }
}

pub type GrpcTraceLayer = tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::GrpcErrorsAsFailures>,
    GrpcMakeSpan,
>;

// Apply with Server::builder().layer(wms_core::grpc::trace_layer())
pub fn trace_layer() -> GrpcTraceLayer {
    tower_http::trace::TraceLayer::new_for_grpc().make_span_with(GrpcMakeSpan)
}
