use boutique::store::sea_orm::{create_sessions_table, create_sessions_user_id_index};
use sea_orm_migration::prelude::*;

use crate::models::{session, user};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Password).string().not_null())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;
        manager.create_table(create_sessions_table::<user::Entity, session::Entity>()).await?;
        manager.create_index(create_sessions_user_id_index::<session::Entity>()).await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
    Password,
    CreatedAt,
}
