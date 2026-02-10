use ::redis::AsyncCommands;
use anyhow::Result;
use axio::{
    AppResult,
    middleware::{compression, cors, request_id, trace, trace_body},
    postgres, redis,
    validation::ValidatedJson,
};
use axum::{
    Router,
    extract::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 1, message = "Can not be empty"))]
    pub username: String,
}

#[derive(Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
}

pub async fn root() -> AppResult<String> {
    let mut con = redis::pool().await?;
    let _: () = con.set_ex("greeting", "Hello, axio-demo!", 10).await?;
    let result: String = con.get("greeting").await?;
    Ok(result)
}

pub async fn create_user(
    ValidatedJson(payload): ValidatedJson<CreateUser>,
) -> AppResult<Json<User>> {
    let user = sqlx::query_as!(
        User,
        r#"insert into users (username) values ($1) returning id, username"#,
        payload.username
    )
    .fetch_one(postgres::pool())
    .await?;
    Ok(Json(user))
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .layer(
            ServiceBuilder::new()
                .layer(compression::compression())
                .layer(request_id::set_request_id())
                .layer(request_id::propagate_request_id())
                .layer(trace::trace())
                .layer(cors::cors())
                .layer(trace_body::trace_body()),
        )
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = axio::config::Config::load("config.toml")?;
    let _worker_guard = axio::app::AppBuilder::new(config)
        .with_router(router)
        .before_run(|| {
            tokio::spawn(async move {
                println!("Running pre-run initialization tasks...");
                Ok(())
            })
        })
        .run()
        .await?;
    Ok(())
}
