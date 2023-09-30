use actix_cors::Cors;
use actix_web::{guard, http, web, App, HttpResponse, HttpServer, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_graphql::{http::GraphiQLSource, EmptySubscription, Object, Schema, SimpleObject};
use async_graphql_actix_web::GraphQL;
use chrono::Utc;
use entity::{
    achievment::{self, Entity as Achievment},
    room::{self, Entity as Room},
    task::{self, Entity as Task},
    user::{self, Entity as User},
    user_achievment::{self, Entity as UserAchievment},
    user_room::{self, Entity as UserRoom},
};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait, ModelTrait,
    QueryFilter, Set,
};
use sha2::Sha256;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use std::collections::HashSet;

async fn index_graphiql() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/").finish()))
}

#[derive(SimpleObject)]
#[graphql(name = "LoginResponse")]
pub struct LoginResponse {
    refresh_token: String,
    access_token: String,
}

pub struct Context {
    pub db: DatabaseConnection,
}

impl Context {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn me(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
    ) -> Result<user::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            println!("{}, {}", claims["exp"], now);
            let id: i32 = claims["id"].parse().unwrap();
            let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

            let mut user = match user {
                Some(user) => user,
                None => return Err(async_graphql::Error::new("Wrong token".to_string())),
            };

            let userAchievments: Vec<user_achievment::Model> =
                user.find_related(UserAchievment).all(&my_ctx.db).await?;

            let achievments: Option<Vec<achievment::Model>> =
                Some(Achievment::find().all(&my_ctx.db).await?);

            let achievments = match achievments {
                Some(achievments) => achievments,
                None => return Err(async_graphql::Error::new("task not found".to_string())),
            };

            let achievments_ids = userAchievments
                .into_iter()
                .map(|model| model.achievment_id)
                .collect::<HashSet<_>>();

            let achievements = achievments
                .into_iter()
                .filter(|model| achievments_ids.contains(&model.id))
                .collect::<Vec<_>>();

            println!("{:?}", achievements);

            user.achievments = achievements;

            return Ok(user);
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in".to_string(),
            ));
        }
    }

    async fn get_users_from_classroom(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i32,
        access_token: String,
    ) -> Result<Vec<user::Model>, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            let user_rooms: Vec<user_room::Model> = user_room::Entity::find()
                .filter(user_room::Column::RoomId.eq(id))
                .all(&my_ctx.db)
                .await?;

            let user_ids: Vec<i32> = user_rooms
                .into_iter()
                .map(|user_room| user_room.user_id)
                .collect();

            let users: Vec<user::Model> = user::Entity::find()
                .filter(user::Column::Id.is_in(user_ids))
                .all(&my_ctx.db)
                .await?;

            println!("{:?}", users);

            Ok(users)
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in".to_string(),
            ));
        }
    }

    async fn get_task(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i32,
        access_token: String,
    ) -> Result<task::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();

        let task: Option<task::Model> = Task::find_by_id(id).one(&my_ctx.db).await?;

        let task = match task {
            Some(task) => task,
            None => return Err(async_graphql::Error::new("task not found".to_string())),
        };

        // You can now access the database connection via `my_ctx.db`
        return Ok(task);
    }

    async fn get_room_tasks(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i32,
        access_token: String,
    ) -> Result<Vec<task::Model>, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();

        let room: Option<room::Model> = Room::find_by_id(id).one(&my_ctx.db).await?;

        let room = match room {
            Some(room) => room,
            None => return Err(async_graphql::Error::new("Wrong room id".to_string())),
        };

        let tasks: Vec<task::Model> = room.find_related(Task).all(&my_ctx.db).await?;

        return Ok(tasks);
    }

    async fn get_my_rooms(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
    ) -> Result<Vec<user_room::Model>, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            println!("{}, {}", claims["exp"], now);
            let id: i32 = claims["id"].parse().unwrap();
            let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

            let user = match user {
                Some(user) => user,
                None => return Err(async_graphql::Error::new("User not found".to_string())),
            };

            let user_rooms: Vec<user_room::Model> =
                user.find_related(UserRoom).all(&my_ctx.db).await?;

            // let rooms: Vec<room::Model> = user_rooms
            //     .into_iter()
            //     .map(|user_room| user_room.find_related(Room).one(&my_ctx.db))
            //     .collect::<Result<Vec<_>, _>>()?;

            Ok(user_rooms)
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in".to_string(),
            ));
        }
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn register(
        &self,
        ctx: &async_graphql::Context<'_>,
        username: String,
        email: String,
        password: String,
        role: i32,
        name: String,
        last_name: String,
        school: String,
    ) -> Result<user::Model, async_graphql::Error> {
        if role == 1 || role == 0 {
            let my_ctx = ctx.data::<Context>().unwrap();
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            let password_hash = match argon2.hash_password(password.as_bytes(), &salt) {
                Ok(hash) => hash.to_string(),
                Err(err) => {
                    return Err(async_graphql::Error::new(err.to_string()));
                }
            };

            let parsed_hash = PasswordHash::new(&password_hash)
                .map_err(|err| async_graphql::Error::new(err.to_string()))?;

            let naive_date_time = Utc::now().naive_utc();

            let user = user::ActiveModel {
                username: Set(username),
                email: Set(email),
                password_hash: Set(parsed_hash.to_string()),
                created_at: Set(naive_date_time),
                updated_at: Set(naive_date_time),
                refresh_token: Set(None),
                role: Set(role),
                name: Set(name),
                last_name: Set(last_name),
                school: Set(school),
                ..Default::default()
            };

            let user: user::Model = user.insert(&my_ctx.db).await?;

            return Ok(user);
        } else {
            return Err(async_graphql::Error::new("Something is wrong".to_string()));
        }
    }

    async fn login(
        &self,
        ctx: &async_graphql::Context<'_>,
        email: String,
        password: String,
    ) -> Result<LoginResponse, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();

        let user: Option<user::Model> = User::find_by_email(email).one(&my_ctx.db).await?;

        let user = match user {
            Some(user) => user,
            None => {
                return Err(async_graphql::Error::new(
                    "Wrong email or password".to_string(),
                ))
            }
        };

        let argon2 = Argon2::default();

        let response = argon2
            .verify_password(
                password.as_bytes(),
                &PasswordHash::new(&user.password_hash).unwrap(),
            )
            .is_ok();

        if response {
            let refresh_key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret") {
                Ok(key) => key,
                Err(err) => return Err(async_graphql::Error::new("internal error".to_string())),
            };

            let access_key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
                Ok(key) => key,
                Err(err) => return Err(async_graphql::Error::new("internal error".to_string())),
            };

            let mut refresh_claims = BTreeMap::new();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize;
            let expiration = now + (1 * 60); // 1 minutes from now
            let expiration = expiration.to_string();
            let expiration2 = now + (60 * 60); // 60 minutes from now
            let expiration2 = expiration2.to_string();

            let id = user.id.to_string();
            let email = user.email.to_string();
            let role = user.role.to_string();

            refresh_claims.insert("sub", "someone");
            refresh_claims.insert("id", &id);
            refresh_claims.insert("email", &email);
            refresh_claims.insert("exp", &expiration2);
            refresh_claims.insert("role", &role);

            let refresh_token = match refresh_claims.clone().sign_with_key(&refresh_key) {
                Ok(token) => token,
                Err(err) => return Err(async_graphql::Error::new("internal error".to_string())),
            };

            let mut access_claims = BTreeMap::new();
            access_claims.insert("sub", "someone");
            access_claims.insert("id", &id);
            access_claims.insert("email", &email);
            access_claims.insert("exp", &expiration);
            access_claims.insert("role", &role);
            let access_token = match access_claims.sign_with_key(&access_key) {
                Ok(token) => token,
                Err(err) => return Err(async_graphql::Error::new("internal error".to_string())),
            };

            user::ActiveModel {
                id: Set(user.id),
                refresh_token: Set(Some(refresh_token.clone())),
                ..Default::default()
            }
            .update(&my_ctx.db)
            .await?;

            Ok(LoginResponse {
                refresh_token,
                access_token,
            })
        } else {
            return Err(async_graphql::Error::new(
                "Wrong email or password".to_string(),
            ));
        }
    }

    async fn create_room(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
        name: String,
    ) -> Result<room::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone"
            && (claims["role"] == "1" || claims["role"] == "2")
            && claims["exp"].parse::<usize>().unwrap() >= now
        {
            let id = claims["id"].parse::<i32>().unwrap();
            let naive_date_time = Utc::now().naive_utc();
            let room = room::ActiveModel {
                created_at: Set(naive_date_time),
                updated_at: Set(naive_date_time),
                name: Set(name),
                owner: Set(id),
                ..Default::default()
            };
            let room: room::Model = room.insert(&my_ctx.db).await?;
            return Ok(room);
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in or you are not a teacher".to_string(),
            ));
        }
    }

    async fn create_task(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
        room_id: i32,
        title: String,
    ) -> Result<task::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone"
            && claims["role"] == "1"
            && claims["exp"].parse::<usize>().unwrap() >= now
        {
            let id = claims["id"].parse::<i32>().unwrap();
            let naive_date_time = Utc::now().naive_utc();
            let task = task::ActiveModel {
                created_at: Set(naive_date_time),
                updated_at: Set(naive_date_time),
                room_id: Set(room_id),
                title: Set(title),
                ..Default::default()
            };
            let task: task::Model = task.insert(&my_ctx.db).await?;
            return Ok(task);
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in or you are not a teacher".to_string(),
            ));
        }
    }

    async fn create_achievment(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
        title: String,
        description: String,
    ) -> Result<achievment::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(b"some-secret2") {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone"
            && claims["role"] == "2"
            && claims["exp"].parse::<usize>().unwrap() >= now
        {
            let naive_date_time = Utc::now().naive_utc();
            let achievment = achievment::ActiveModel {
                created_at: Set(naive_date_time),
                updated_at: Set(naive_date_time),
                title: Set(title),
                description: Set(description),
                ..Default::default()
            };
            let achievment: achievment::Model = achievment.insert(&my_ctx.db).await?;
            return Ok(achievment);
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in or you are not a teacher".to_string(),
            ));
        }
    }

    async fn add_achievement(
        &self,
        ctx: &async_graphql::Context<'_>,
        user_id: i32,
        achievment_id: i32,
    ) -> Result<user_achievment::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let userAchievement = user_achievment::ActiveModel {
            user_id: Set(user_id),
            achievment_id: Set(achievment_id),
            ..Default::default()
        };

        let achievment: user_achievment::Model = userAchievement.insert(&my_ctx.db).await?;
        Ok(achievment)
    }

    async fn add_to_room(
        &self,
        ctx: &async_graphql::Context<'_>,
        user_id: i32,
        room_id: i32,
    ) -> Result<user_room::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let user_room = user_room::ActiveModel {
            user_id: Set(user_id),
            room_id: Set(room_id),
            ..Default::default()
        };

        let user_room: user_room::Model = user_room.insert(&my_ctx.db).await?;
        Ok(user_room)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();
    let db: DatabaseConnection =
        Database::connect("postgres://postgres:password@localhost:5432/stem")
            .await
            .expect("error with connection");

    Migrator::up(&db, None).await.expect("migration ban");

    println!("GraphiQL IDE: http://localhost:8000");

    HttpServer::new(move || {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
            .data(Context::new(db.clone())) // add the context here
            .finish();
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:3000")
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:8000")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(cors)
            .service(
                web::resource("/")
                    .guard(guard::Post())
                    .to(GraphQL::new(schema)),
            )
            .service(web::resource("/").guard(guard::Get()).to(index_graphiql))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}