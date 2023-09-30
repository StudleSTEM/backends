use async_graphql::SimpleObject;
use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, SimpleObject)]
#[sea_orm(table_name = "achievment")]
#[graphql(name = "achievment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub title: String,
    pub description: String,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_achievment::Entity")]
    UserAchievment,
}

impl Related<super::user_achievment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAchievment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
