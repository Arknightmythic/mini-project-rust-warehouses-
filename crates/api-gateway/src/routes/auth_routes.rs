use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::user::v1 as user_v1;

use crate::dto::{LoginJson, UserJson};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[derive(Deserialize)]
struct RegisterBody {
    name: String,
    email: String,
    password: String,
    phone: Option<String>,
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> AppResult<Json<UserJson>> {
    let mut client = state.users.clone();

    let response = client
        .register(Request::new(user_v1::RegisterRequest {
            name: body.name,
            email: body.email,
            password: body.password,
            phone: body.phone,
        }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(response.into_inner().into()))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<LoginJson>> {
    let mut client = state.users.clone();

    let response = client
        .login(Request::new(user_v1::LoginRequest {
            email: body.email,
            password: body.password,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    let user = response
        .user
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("login response missing user")))?;

    Ok(Json(LoginJson {
        token: response.token,
        user: user.into(),
    }))
}
