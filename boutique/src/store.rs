use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, IntoActiveModel, ModelTrait, QueryFilter,
};

use crate::models::user;

/// implemented on the app's user `Model`. boutique derives every query it
/// needs from the entity and the three column names.
pub trait AuthUser:
    ModelTrait<Entity = Self::UserEntity>
    + FromQueryResult
    + IntoActiveModel<Self::Active>
    + Clone
    + Send
    + Sync
    + 'static
{
    type UserEntity: EntityTrait<Model = Self, Column = Self::UserColumn>;
    type UserColumn: ColumnTrait;
    type Active: ActiveModelTrait<Entity = Self::UserEntity> + ActiveModelBehavior + Send;

    const ID: Self::UserColumn;
    const EMAIL: Self::UserColumn;
    const PASSWORD: Self::UserColumn;

    fn id(&self) -> &str;
    fn email(&self) -> &str;
    fn password_hash(&self) -> &str;
}

pub(crate) async fn find_by_id<U: AuthUser>(db: &DatabaseConnection, id: &str) -> Result<Option<U>, DbErr> {
    U::UserEntity::find().filter(U::ID.eq(id)).one(db).await
}

pub(crate) async fn find_by_email<U: AuthUser>(db: &DatabaseConnection, email: &str) -> Result<Option<U>, DbErr> {
    U::UserEntity::find().filter(U::EMAIL.eq(email)).one(db).await
}

pub(crate) async fn set_password_hash<U: AuthUser>(db: &DatabaseConnection, user: U, hash: String) -> Result<U, DbErr> {
    let mut active = user.into_active_model();
    active.set(U::PASSWORD, hash.into());
    active.update(db).await
}

impl AuthUser for user::Model {
    type UserEntity = user::Entity;
    type UserColumn = user::Column;
    type Active = user::ActiveModel;

    const ID: user::Column = user::Column::Id;
    const EMAIL: user::Column = user::Column::Email;
    const PASSWORD: user::Column = user::Column::Password;

    fn id(&self) -> &str {
        &self.id
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn password_hash(&self) -> &str {
        &self.password
    }
}
