use boutique::{
    AuthUser, chrono, ids,
    password::{self, HashError},
    store::sea_orm::{SeaOrmStore, UserEntity},
};
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub email: String,
    pub password: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl UserEntity for Entity {
    const ID: Column = Column::Id;
    const EMAIL: Column = Column::Email;
    const PASSWORD_HASH: Column = Column::Password;
}

impl AuthUser for Model {
    fn id(&self) -> &str {
        &self.id
    }

    fn password_hash(&self) -> &str {
        &self.password
    }
}

pub type Store = SeaOrmStore<Entity, super::session::Entity>;

pub async fn new(email: &str, plain_password: &str) -> Result<ActiveModel, HashError> {
    Ok(ActiveModel {
        id: Set(ids::new()),
        email: Set(email.to_string()),
        password: Set(password::hash_off_runtime(plain_password).await?),
        created_at: Set(chrono::Utc::now().into()),
    })
}
