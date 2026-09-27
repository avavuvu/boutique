mod create_users;
mod create_refresh_tokens;

use sea_orm::{DatabaseConnection, DbErr};
use sea_orm_migration::{MigrationTrait, MigratorTrait, async_trait};

/// migrations for the tables boutique owns. append these to the app's migrator
/// before any app migration that references `users`.
pub fn all() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(create_users::Migration),
        Box::new(create_refresh_tokens::Migration),
    ]
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        all()
    }
}

pub async fn up(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}
