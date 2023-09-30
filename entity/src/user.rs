use async_graphql::SimpleObject;
use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, SimpleObject)]
#[sea_orm(table_name = "user")]
#[graphql(name = "UserModel")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(column_name = "username")]
    pub username: String,

    pub name: String,

    pub last_name: String,

    pub school: String,

    pub score: i32,

    pub class: String,
    #[sea_orm(column_name = "email")]
    pub email: String,

    #[sea_orm(column_name = "role")]
    pub role: i32,

    #[graphql(visible = false)]
    pub password_hash: String,

    #[graphql(visible = false)]
    pub refresh_token: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    #[sea_orm(ignore)]
    pub achievments: Vec<super::achievment::Model>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_room::Entity")]
    UserRoom,
    #[sea_orm(has_many = "super::room::Entity")]
    Room,
    #[sea_orm(has_many = "super::user_achievment::Entity")]
    UserAchievment,
}

impl Related<super::user_room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserRoom.def()
    }
}

impl Related<super::room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Room.def()
    }
}

impl Related<super::user_achievment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAchievment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub fn find_by_email(email: String) -> Select<Entity> {
        Self::find().filter(Column::Email.eq(email))
    }
}
