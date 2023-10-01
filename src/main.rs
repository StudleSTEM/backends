use actix_cors::Cors;
use actix_web::{guard, http, web, App, HttpResponse, HttpServer, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_graphql::{http::GraphiQLSource, EmptySubscription, Object, Schema, SimpleObject};
use async_graphql_actix_web::GraphQL;
use chrono::Utc;
use dotenvy::dotenv;
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
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter, Set, TryIntoModel,
};
use sha2::Sha256;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use std::collections::HashSet;

const ACCESS_EXPIRATION: usize = 5;
const REFRESH_EXPIRATION: usize = 180;

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

struct Context {
    db: DatabaseConnection,
    acs_key: String,
    refr_key: String,
}

impl Context {
    fn new(db: DatabaseConnection, acs_key: String, refr_key: String) -> Self {
        Self {
            db,
            acs_key,
            refr_key,
        }
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
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
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

            let user_achievments: Vec<user_achievment::Model> =
                user.find_related(UserAchievment).all(&my_ctx.db).await?;

            let ids: Vec<i32> = user_achievments
                .iter()
                .map(|achievment| achievment.achievment_id)
                .collect();

            let achs: Option<Vec<achievment::Model>> = Some(
                Achievment::find()
                    .filter(achievment::Column::Id.is_in(ids))
                    .all(&my_ctx.db)
                    .await?,
            );

            let achs = match achs {
                Some(achs) => achs,
                None => return Err(async_graphql::Error::new("user not found".to_string())),
            };

            let user_rooms: Vec<user_room::Model> =
                user.find_related(UserRoom).all(&my_ctx.db).await?;

            let ids: Vec<i32> = user_rooms.iter().map(|room| room.room_id).collect();

            let rooms: Option<Vec<room::Model>> = Some(
                Room::find()
                    .filter(room::Column::Id.is_in(ids))
                    .all(&my_ctx.db)
                    .await?,
            );

            let rooms = match rooms {
                Some(rooms) => rooms,
                None => return Err(async_graphql::Error::new("error".to_string())),
            };

            user.achievments = achs;
            user.rooms = rooms;

            return Ok(user);
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

    async fn get_user(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: i32,
    ) -> Result<user::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();

        let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

        let mut user = match user {
            Some(user) => user,
            None => return Err(async_graphql::Error::new("user not found".to_string())),
        };

        let achievments: Option<Vec<user_achievment::Model>> = Some(
            UserAchievment::find()
                .filter(user_achievment::Column::UserId.eq(id))
                .all(&my_ctx.db)
                .await?,
        );

        let achievments = match achievments {
            Some(achievments) => achievments,
            None => return Err(async_graphql::Error::new("room not found".to_string())),
        };

        let ids: Vec<i32> = achievments
            .iter()
            .map(|achievment| achievment.achievment_id)
            .collect();

        let achs: Option<Vec<achievment::Model>> = Some(
            Achievment::find()
                .filter(achievment::Column::Id.is_in(ids))
                .all(&my_ctx.db)
                .await?,
        );

        let achs = match achs {
            Some(achs) => achs,
            None => return Err(async_graphql::Error::new("user not found".to_string())),
        };

        user.achievments = achs;

        Ok(user)
    }

    async fn get_room(
        &self,
        ctx: &async_graphql::Context<'_>,
        room_id: i32,
        access_token: String,
    ) -> Result<room::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            let id = claims["id"].parse::<i32>().unwrap();
            let rooms: Option<Vec<user_room::Model>> = Some(
                UserRoom::find()
                    .filter(user_room::Column::UserId.eq(id))
                    .all(&my_ctx.db)
                    .await?,
            );

            let rooms = match rooms {
                Some(rooms) => rooms,
                None => return Err(async_graphql::Error::new("room not found".to_string())),
            };

            let ids: Vec<i32> = rooms.iter().map(|room| room.room_id).collect();

            println!("{:?}", ids);

            if ids.contains(&room_id) {
                let rooms: Option<Vec<user_room::Model>> = Some(
                    UserRoom::find()
                        .filter(user_room::Column::RoomId.eq(room_id))
                        .all(&my_ctx.db)
                        .await?,
                );

                let rooms = match rooms {
                    Some(rooms) => rooms,
                    None => return Err(async_graphql::Error::new("room not found".to_string())),
                };

                let ids: Vec<i32> = rooms.iter().map(|room| room.user_id).collect();

                let users: Option<Vec<user::Model>> = Some(
                    User::find()
                        .filter(user::Column::Id.is_in(ids))
                        .all(&my_ctx.db)
                        .await?,
                );

                let tasks: Option<Vec<task::Model>> = Some(Task::find().all(&my_ctx.db).await?);

                let users = match users {
                    Some(users) => users,
                    None => return Err(async_graphql::Error::new("internal error".to_string())),
                };

                let room: Option<room::Model> = Room::find_by_id(room_id).one(&my_ctx.db).await?;

                let mut room = match room {
                    Some(room) => room,
                    None => return Err(async_graphql::Error::new("room not found".to_string())),
                };

                room.users = users;
                room.tasks = tasks.unwrap();

                Ok(room)
            } else {
                return Err(async_graphql::Error::new(
                    "you do not exist in this room".to_string(),
                ));
            }
        } else {
            return Err(async_graphql::Error::new(
                "you are not loged in".to_string(),
            ));
        }
    }

    // async fn get_my_rooms(
    //     &self,
    //     ctx: &async_graphql::Context<'_>,
    //     access_token: String,
    // ) -> Result<Vec<room::Model>, async_graphql::Error> {
    //     let my_ctx = ctx.data::<Context>().unwrap();
    //     let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
    //         Ok(key) => key,
    //         Err(err) => return Err(async_graphql::Error::new(err.to_string())),
    //     };
    //     let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
    //         Ok(res) => res,
    //         Err(err) => return Err(async_graphql::Error::new(err.to_string())),
    //     };
    //     let now = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .unwrap()
    //         .as_secs() as usize;
    //     if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
    //         let id: i32 = claims["id"].parse().unwrap();
    //         let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

    //         let user = match user {
    //             Some(user) => user,
    //             None => return Err(async_graphql::Error::new("User not found".to_string())),
    //         };

    //         Ok(rooms)
    //     } else {
    //         return Err(async_graphql::Error::new(
    //             "you are not loged in".to_string(),
    //         ));
    //     }
    // }
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
        class: String,
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
                class: Set(class),
                score: Set(0),
                avatar_url: Set(None),
                ..Default::default()
            };

            let user: user::Model = user.insert(&my_ctx.db).await?;

            return Ok(user);
        } else {
            return Err(async_graphql::Error::new("Something is wrong".to_string()));
        }
    }

    async fn refresh(
        &self,
        ctx: &async_graphql::Context<'_>,
        refresh_token: String,
    ) -> Result<LoginResponse, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();

        let refresh_key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.refr_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };

        let claims: BTreeMap<String, String> =
            match refresh_token.clone().verify_with_key(&refresh_key) {
                Ok(res) => res,
                Err(err) => return Err(async_graphql::Error::new(err.to_string())),
            };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            let id = claims["id"].parse::<i32>().unwrap();
            let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

            let user = match user {
                Some(user) => user,
                None => return Err(async_graphql::Error::new("Wrong token".to_string())),
            };

            if user.refresh_token == Some(refresh_token.clone()) {
                let access_key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes())
                {
                    Ok(key) => key,
                    Err(err) => return Err(async_graphql::Error::new(err.to_string())),
                };

                let mut refresh_claims = BTreeMap::new();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as usize;
                let expiration = now + (ACCESS_EXPIRATION * 60); // 1 minutes from now
                let expiration = expiration.to_string();
                let expiration2 = now + (REFRESH_EXPIRATION * 60); // 60 minutes from now
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
                    Err(err) => return Err(async_graphql::Error::new(err.to_string())),
                };

                let mut access_claims = BTreeMap::new();
                access_claims.insert("sub", "someone");
                access_claims.insert("id", &id);
                access_claims.insert("email", &email);
                access_claims.insert("exp", &expiration);
                access_claims.insert("role", &role);

                let access_token = match access_claims.sign_with_key(&access_key) {
                    Ok(token) => token,
                    Err(err) => return Err(async_graphql::Error::new(err.to_string())),
                };
                let naive_date_time = Utc::now().naive_utc();

                user::ActiveModel {
                    id: Set(user.id),
                    refresh_token: Set(Some(refresh_token.clone())),
                    updated_at: Set(naive_date_time),
                    ..Default::default()
                }
                .update(&my_ctx.db)
                .await?;

                return Ok(LoginResponse {
                    refresh_token,
                    access_token,
                });
            } else {
                return Err(async_graphql::Error::new("Wrong token".to_string()));
            }
        } else {
            return Err(async_graphql::Error::new("Wrong token".to_string()));
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
            let refresh_key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.refr_key.as_bytes()) {
                Ok(key) => key,
                Err(err) => return Err(async_graphql::Error::new(err.to_string())),
            };

            let access_key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
                Ok(key) => key,
                Err(err) => return Err(async_graphql::Error::new(err.to_string())),
            };

            let mut refresh_claims = BTreeMap::new();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize;
            let expiration = now + (ACCESS_EXPIRATION * 60); // 1 minutes from now
            let expiration = expiration.to_string();
            let expiration2 = now + (REFRESH_EXPIRATION * 60); // 60 minutes from now
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
                Err(err) => return Err(async_graphql::Error::new(err.to_string())),
            };

            let mut access_claims = BTreeMap::new();
            access_claims.insert("sub", "someone");
            access_claims.insert("id", &id);
            access_claims.insert("email", &email);
            access_claims.insert("exp", &expiration);
            access_claims.insert("role", &role);
            let access_token = match access_claims.sign_with_key(&access_key) {
                Ok(token) => token,
                Err(err) => return Err(async_graphql::Error::new(err.to_string())),
            };

            let naive_date_time = Utc::now().naive_utc();

            user::ActiveModel {
                id: Set(user.id),
                refresh_token: Set(Some(refresh_token.clone())),
                updated_at: Set(naive_date_time),
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
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
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

    async fn edit(
        &self,
        ctx: &async_graphql::Context<'_>,
        access_token: String,
        school: Option<String>,
        name: Option<String>,
        last_name: Option<String>,
        class: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<user::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone" && claims["exp"].parse::<usize>().unwrap() >= now {
            let id = claims["id"].parse::<i32>().unwrap();
            let naive_date_time = Utc::now().naive_utc();

            let user: Option<user::Model> = User::find_by_id(id).one(&my_ctx.db).await?;

            let user = match user {
                Some(user) => user,
                None => return Err(async_graphql::Error::new("Wrong token".to_string())),
            };

            let mut newuser: user::ActiveModel = user.into();

            match school {
                Some(school) => {
                    newuser.school = Set(school);
                }
                None => (),
            }

            match name {
                Some(name) => {
                    newuser.name = Set(name);
                }
                None => (),
            }

            match last_name {
                Some(last_name) => {
                    newuser.last_name = Set(last_name);
                }
                None => (),
            }

            match class {
                Some(class) => {
                    newuser.class = Set(class);
                }
                None => (),
            }

            match avatar_url {
                Some(avatar_url) => {
                    newuser.avatar_url = Set(Some(avatar_url));
                }
                None => (),
            }

            newuser.updated_at = Set(naive_date_time);

            newuser.clone().update(&my_ctx.db).await?;

            let updated_user: user::Model = newuser.try_into_model().unwrap();

            Ok(updated_user)
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
        content: String,
    ) -> Result<task::Model, async_graphql::Error> {
        let my_ctx = ctx.data::<Context>().unwrap();
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        if claims["sub"] == "someone"
            && (claims["role"] == "1" || claims["role"] == "2")
            && claims["exp"].parse::<usize>().unwrap() >= now
        {
            let naive_date_time = Utc::now().naive_utc();
            let task = task::ActiveModel {
                created_at: Set(naive_date_time),
                updated_at: Set(naive_date_time),
                room_id: Set(room_id),
                title: Set(title),
                content: Set(content),
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
        let key: Hmac<Sha256> = match Hmac::new_from_slice(my_ctx.acs_key.as_bytes()) {
            Ok(key) => key,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
        };
        let claims: BTreeMap<String, String> = match access_token.verify_with_key(&key) {
            Ok(res) => res,
            Err(err) => return Err(async_graphql::Error::new(err.to_string())),
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
        let user_achievement = user_achievment::ActiveModel {
            user_id: Set(user_id),
            achievment_id: Set(achievment_id),
            string: Set(format!("{}-{}", user_id, achievment_id)),
            ..Default::default()
        };

        let user: Option<user::Model> = User::find_by_id(user_id).one(&my_ctx.db).await?;

        let user = match user {
            Some(user) => user,
            None => return Err(async_graphql::Error::new("Wrong token".to_string())),
        };

        let mut newuser: user::ActiveModel = user.into();
        let naive_date_time = Utc::now().naive_utc();

        newuser.score = Set(newuser.score.unwrap() + 1);
        newuser.updated_at = Set(naive_date_time);

        newuser.update(&my_ctx.db).await?;

        let achievment: user_achievment::Model = user_achievement.insert(&my_ctx.db).await?;

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
            string: Set(format!("{}-{}", user_id, room_id)),
            ..Default::default()
        };

        let user_room: user_room::Model = user_room.insert(&my_ctx.db).await?;
        Ok(user_room)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect(".env file not found");
    let db_url = dotenvy::var("DATABASE_URL").expect("HOME environment variable not found");
    let refr_key = dotenvy::var("REFRESH_KEY").expect("HOME environment variable not found");
    let acs_key = dotenvy::var("ACCESS_KEY").expect("HOME environment variable not found");
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();
    let db: DatabaseConnection = Database::connect(db_url)
        .await
        .expect("error with connection");

    Migrator::up(&db, None).await.expect("migration ban");

    println!("GraphiQL IDE: http://localhost:8000");

    HttpServer::new(move || {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
            .data(Context::new(db.clone(), acs_key.clone(), refr_key.clone())) // add the context here
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
