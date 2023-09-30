use async_graphql::SimpleObject;
use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, SimpleObject)]
#[sea_orm(table_name = "room")]
#[graphql(name = "roomModel")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub owner: i32,

    #[sea_orm(column_name = "name")]
    pub name: String,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    #[sea_orm(ignore)]
    pub tasks: Vec<super::task::Model>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::task::Entity")]
    Task,
    #[sea_orm(has_many = "super::user_room::Entity")]
    UserRoom,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Owner",
        to = "super::user::Column::Id"
    )]
    User,
}

// `Related` trait has to be implemented by hand
impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl Related<super::user_room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserRoom.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
