use sea_orm_migration::prelude::*;

use super::create_users::Users;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00000000_000002_boutique_create_refresh_tokens"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RefreshTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RefreshTokens::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(RefreshTokens::UserId).string().not_null())
                    .col(ColumnDef::new(RefreshTokens::Token).string().not_null().unique_key())
                    .col(ColumnDef::new(RefreshTokens::ExpiresAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(RefreshTokens::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(RefreshTokens::Table, RefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RefreshTokens::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    Token,
    ExpiresAt,
    CreatedAt,
}
