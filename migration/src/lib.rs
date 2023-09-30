pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230930_195109_update1;
mod m20230930_195236_update2;
mod m20230930_195346_update3;
mod m20230930_195548_update4;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230930_195109_update1::Migration),
            Box::new(m20230930_195236_update2::Migration),
            Box::new(m20230930_195346_update3::Migration),
            Box::new(m20230930_195548_update4::Migration),
        ]
    }
}
