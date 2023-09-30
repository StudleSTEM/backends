pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230930_080511_update1;
mod m20230930_081315_update2;
mod m20230930_081503_update3;
mod m20230930_082749_update4;
mod m20230930_090923_update5;
mod m20230930_093017_update6;
mod m20230930_093126_update7;
mod m20230930_093331_update8;
mod m20230930_094646_update9;
mod m20230930_100432_update10;
mod m20230930_102215_update11;
mod m20230930_105117_update11;
mod m20230930_105636_update12;
mod m20230930_114305_update13;
mod m20230930_114450_update14;
mod m20230930_115447_update15;
mod m20230930_132541_update16;
mod m20230930_132830_update17;
mod m20230930_142315_update19;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230930_080511_update1::Migration),
            Box::new(m20230930_081315_update2::Migration),
            Box::new(m20230930_081503_update3::Migration),
            Box::new(m20230930_082749_update4::Migration),
            Box::new(m20230930_090923_update5::Migration),
            Box::new(m20230930_093017_update6::Migration),
            Box::new(m20230930_093126_update7::Migration),
            Box::new(m20230930_093331_update8::Migration),
            Box::new(m20230930_094646_update9::Migration),
            Box::new(m20230930_100432_update10::Migration),
            Box::new(m20230930_102215_update11::Migration),
            Box::new(m20230930_105117_update11::Migration),
            Box::new(m20230930_105636_update12::Migration),
            Box::new(m20230930_114305_update13::Migration),
            Box::new(m20230930_114450_update14::Migration),
            Box::new(m20230930_115447_update15::Migration),
            Box::new(m20230930_132541_update16::Migration),
            Box::new(m20230930_132830_update17::Migration),
            Box::new(m20230930_142315_update19::Migration),
        ]
    }
}
