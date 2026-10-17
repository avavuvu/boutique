use boutique::store::sea_orm::SessionEntity;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_hash: String,
    pub user_id: String,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl SessionEntity for Entity {
    const TOKEN_HASH: Column = Column::TokenHash;
    const USER_ID: Column = Column::UserId;
    const CREATED_AT: Column = Column::CreatedAt;
    const EXPIRES_AT: Column = Column::ExpiresAt;
}
