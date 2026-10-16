use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, JoinType, ModelTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};

use crate::{
    auth::store::{AuthStore, NewSession},
    error::AppError,
};

impl From<DbErr> for AppError {
    fn from(error: DbErr) -> Self {
        AppError::internal("database", error)
    }
}

pub trait UserEntity: EntityTrait {
    const ID: Self::Column;
    const EMAIL: Self::Column;
    const PASSWORD_HASH: Self::Column;
}

pub trait SessionEntity: EntityTrait {
    const TOKEN_HASH: Self::Column;
    const USER_ID: Self::Column;
    const CREATED_AT: Self::Column;
    const EXPIRES_AT: Self::Column;
}

pub struct SeaOrmStore<U, S> {
    db: DatabaseConnection,
    entities: PhantomData<fn() -> (U, S)>,
}

impl<U, S> SeaOrmStore<U, S> {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db, entities: PhantomData }
    }
}

impl<U, S> Clone for SeaOrmStore<U, S> {
    fn clone(&self) -> Self {
        Self::new(self.db.clone())
    }
}

impl<U, S> AuthStore<U::Model> for SeaOrmStore<U, S>
where
    U: UserEntity,
    S: SessionEntity,
    U::Model: IntoActiveModel<U::ActiveModel> + Sync,
    U::ActiveModel: Send,
    S::Model: IntoActiveModel<S::ActiveModel>,
    S::ActiveModel: Send,
{
    type Error = DbErr;

    async fn find_user_by_email(&self, email: &str) -> Result<Option<U::Model>, DbErr> {
        U::find().filter(U::EMAIL.eq(email)).one(&self.db).await
    }

    async fn find_user_by_id(&self, id: &str) -> Result<Option<U::Model>, DbErr> {
        U::find().filter(U::ID.eq(id)).one(&self.db).await
    }

    async fn create_session(&self, new: NewSession) -> Result<(), DbErr> {
        let mut active = <S::ActiveModel as ActiveModelTrait>::default();
        active.try_set(S::TOKEN_HASH, new.token_hash.into())?;
        active.try_set(S::USER_ID, new.user_id.into())?;
        active.try_set(S::CREATED_AT, new.created_at.fixed_offset().into())?;
        active.try_set(S::EXPIRES_AT, new.expires_at.fixed_offset().into())?;
        S::insert(active).exec_without_returning(&self.db).await?;
        Ok(())
    }

    async fn find_session_user(&self, token_hash: &str, now: DateTime<Utc>) -> Result<Option<U::Model>, DbErr> {
        U::find()
            .join_rev(JoinType::InnerJoin, S::belongs_to(U::default()).from(S::USER_ID).to(U::ID).into())
            .filter(S::TOKEN_HASH.eq(token_hash))
            .filter(S::EXPIRES_AT.gt(now))
            .one(&self.db)
            .await
    }

    async fn delete_session(&self, token_hash: &str) -> Result<(), DbErr> {
        S::delete_many().filter(S::TOKEN_HASH.eq(token_hash)).exec(&self.db).await?;
        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<(), DbErr> {
        S::delete_many().filter(S::USER_ID.eq(user_id)).exec(&self.db).await?;
        Ok(())
    }

    async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> Result<(), DbErr> {
        S::delete_many().filter(S::EXPIRES_AT.lte(now)).exec(&self.db).await?;
        Ok(())
    }

    async fn reset_password(&self, user: U::Model, password_hash: String) -> Result<U::Model, DbErr> {
        let transaction = self.db.begin().await?;

        S::delete_many().filter(S::USER_ID.eq(user.get(U::ID))).exec(&transaction).await?;

        let mut active = user.into_active_model();
        active.try_set(U::PASSWORD_HASH, password_hash.into())?;
        let user = active.update(&transaction).await?;

        transaction.commit().await?;
        Ok(user)
    }
}
