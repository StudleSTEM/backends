use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::Email).string().unique_key().not_null())
                    .col(ColumnDef::new(User::Role).integer().not_null())
                    .col(ColumnDef::new(User::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(User::UpdatedAt).date_time().not_null())
                    .col(ColumnDef::new(User::PasswordHash).string().not_null())
                    .col(ColumnDef::new(User::RefreshToken).string())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Room::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Room::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Room::Name).string().not_null())
                    .col(ColumnDef::new(Room::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Room::UpdatedAt).date_time().not_null())
                    .col(ColumnDef::new(Room::Owner).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-room-user_id")
                            .from(Room::Table, Room::Owner)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Task::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Task::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Task::RoomId).integer().not_null())
                    .col(ColumnDef::new(Task::Title).string().not_null())
                    .col(ColumnDef::new(Task::Content).string().not_null())
                    .col(ColumnDef::new(Task::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Task::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-task-room_id")
                            .from(Task::Table, Task::RoomId)
                            .to(Room::Table, Room::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(UserRoom::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRoom::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserRoom::UserId).integer().not_null())
                    .col(ColumnDef::new(UserRoom::RoomId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user_room-user_id")
                            .from(UserRoom::Table, UserRoom::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user_room-room_id")
                            .from(UserRoom::Table, UserRoom::RoomId)
                            .to(Room::Table, Room::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Room::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Task::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserRoom::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Email,
    Role,
    CreatedAt,
    UpdatedAt,
    PasswordHash,
    RefreshToken,
}

#[derive(DeriveIden)]
enum Task {
    Table,
    Id,
    Title,
    Content,
    RoomId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Room {
    Table,
    Id,
    Name,
    Owner,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserRoom {
    Table,
    Id,
    UserId,
    RoomId,
}
