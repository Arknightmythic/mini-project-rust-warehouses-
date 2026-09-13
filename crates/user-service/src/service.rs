use std::sync::Arc;

use sqlx::PgPool;
use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_proto::user::v1::user_service_server::UserService;
use wms_proto::user::v1::{
    AssignRoleRequest, CreateRoleRequest, DeleteRoleRequest, DeleteUserRequest, GetRoleRequest,
    GetUserRequest, ListRolesRequest, ListRolesResponse, ListUserRolesRequest, ListUsersRequest,
    ListUsersByRoleRequest, ListUsersResponse, LoginRequest, LoginResponse, RegisterRequest, RemoveRoleRequest, Role,
    UpdateRoleRequest, UpdateUserRequest, User,
};

use crate::config::ServiceConfig;
use crate::models;
use crate::repositories::{role_repository, user_repository, user_role_repository};
use crate::utils::{jwt, password};

// Holds only Clone + Send + Sync values. Never park an sqlx::Transaction in this
// struct: the generated trait requires Send + Sync, and violating it produces one
// of the 60-line async errors.
pub struct UserGrpcService {
    pool: PgPool,
    config: Arc<ServiceConfig>,
}

impl UserGrpcService {
    pub fn new(pool: PgPool, config: Arc<ServiceConfig>) -> Self {
        Self { pool, config }
    }
}

fn to_proto_user(user: models::user::User) -> User {
    User {
        id: user.id,
        name: user.name,
        email: user.email,
        photo: user.photo,
        phone: user.phone,
        created_at: wms_proto::opt_timestamp(user.created_at),
        updated_at: wms_proto::opt_timestamp(user.updated_at),
    }
}

fn to_proto_role(role: models::role::Role) -> Role {
    Role {
        id: role.id,
        name: role.name,
        created_at: wms_proto::opt_timestamp(role.created_at),
        updated_at: wms_proto::opt_timestamp(role.updated_at),
    }
}

#[tonic::async_trait]
impl UserService for UserGrpcService {
    #[tracing::instrument(skip_all)]
    async fn register(&self, request: Request<RegisterRequest>) -> Result<Response<User>, Status> {
        let req = request.into_inner();

        if req.name.trim().is_empty() || req.email.trim().is_empty() {
            return Err(AppError::Validation("name and email are required".to_string()).into());
        }
        if req.password.len() < 8 {
            return Err(
                AppError::Validation("password must be at least 8 characters".to_string()).into(),
            );
        }

        let password_hash = password::hash_password(&req.password)?;
        let user = user_repository::create(
            &self.pool,
            &req.name,
            &req.email,
            &password_hash,
            req.phone.as_deref(),
        )
        .await?;

        Ok(Response::new(to_proto_user(user)))
    }

    #[tracing::instrument(skip_all)]
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let user = user_repository::find_by_email(&self.pool, &req.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if !password::verify_password(&req.password, &user.password)? {
            return Err(AppError::Unauthorized.into());
        }

        let roles = user_repository::list_role_names(&self.pool, user.id).await?;
        let token = jwt::generate_token(&self.config, user.id, &user.email, roles)?;

        Ok(Response::new(LoginResponse {
            token,
            user: Some(to_proto_user(user)),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let user = user_repository::find_by_id(&self.pool, request.into_inner().id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

        Ok(Response::new(to_proto_user(user)))
    }

    #[tracing::instrument(skip_all)]
    async fn list_users(
        &self,
        _request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let users = user_repository::list(&self.pool).await?;

        Ok(Response::new(ListUsersResponse {
            users: users.into_iter().map(to_proto_user).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn list_users_by_role(
        &self,
        request: Request<ListUsersByRoleRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let users = user_repository::list_by_role(&self.pool, &request.into_inner().role).await?;

        Ok(Response::new(ListUsersResponse {
            users: users.into_iter().map(to_proto_user).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<User>, Status> {
        let req = request.into_inner();

        let user = user_repository::update(
            &self.pool,
            req.id,
            req.name.as_deref(),
            req.phone.as_deref(),
            req.photo.as_deref(),
        )
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

        Ok(Response::new(to_proto_user(user)))
    }

    #[tracing::instrument(skip_all)]
    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<()>, Status> {
        let affected = user_repository::delete(&self.pool, request.into_inner().id).await?;
        if affected == 0 {
            return Err(AppError::NotFound("user not found".to_string()).into());
        }

        Ok(Response::new(()))
    }

    #[tracing::instrument(skip_all)]
    async fn list_user_roles(
        &self,
        request: Request<ListUserRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let roles =
            user_role_repository::list_roles_for_user(&self.pool, request.into_inner().user_id)
                .await?;

        Ok(Response::new(ListRolesResponse {
            roles: roles.into_iter().map(to_proto_role).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn assign_role(
        &self,
        request: Request<AssignRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        user_role_repository::assign(&self.pool, req.user_id, req.role_id).await?;

        Ok(Response::new(()))
    }

    #[tracing::instrument(skip_all)]
    async fn remove_role(
        &self,
        request: Request<RemoveRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let affected = user_role_repository::remove(&self.pool, req.user_id, req.role_id).await?;
        if affected == 0 {
            return Err(AppError::NotFound("user role not found".to_string()).into());
        }

        Ok(Response::new(()))
    }

    #[tracing::instrument(skip_all)]
    async fn list_roles(
        &self,
        _request: Request<ListRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let roles = role_repository::list(&self.pool).await?;

        Ok(Response::new(ListRolesResponse {
            roles: roles.into_iter().map(to_proto_role).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn get_role(&self, request: Request<GetRoleRequest>) -> Result<Response<Role>, Status> {
        let role = role_repository::find_by_id(&self.pool, request.into_inner().id)
            .await?
            .ok_or_else(|| AppError::NotFound("role not found".to_string()))?;

        Ok(Response::new(to_proto_role(role)))
    }

    #[tracing::instrument(skip_all)]
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<Role>, Status> {
        let req = request.into_inner();

        if req.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".to_string()).into());
        }

        let role = role_repository::create(&self.pool, &req.name).await?;

        Ok(Response::new(to_proto_role(role)))
    }

    #[tracing::instrument(skip_all)]
    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<Role>, Status> {
        let req = request.into_inner();

        let role = role_repository::update(&self.pool, req.id, &req.name)
            .await?
            .ok_or_else(|| AppError::NotFound("role not found".to_string()))?;

        Ok(Response::new(to_proto_role(role)))
    }

    #[tracing::instrument(skip_all)]
    async fn delete_role(
        &self,
        request: Request<DeleteRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let affected = role_repository::delete(&self.pool, request.into_inner().id).await?;
        if affected == 0 {
            return Err(AppError::NotFound("role not found".to_string()).into());
        }

        Ok(Response::new(()))
    }
}
