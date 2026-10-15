use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, JoinType, ModelTrait,
    QueryFilter, QuerySelect, TransactionTrait,
    sea_query::{
        Alias, ColumnDef, ForeignKey, ForeignKeyAction, Index, IndexCreateStatement, IntoIden, Table,
        TableCreateStatement,
    },
};

use crate::store::{AuthStore, NewSession};

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

fn set_timestamp<A: ActiveModelTrait>(
    active: &mut A,
    column: <A::Entity as EntityTrait>::Column,
    at: DateTime<Utc>,
) -> Result<(), DbErr>
where
    <A::Entity as EntityTrait>::Column: Copy,
{
    active.try_set(column, at.fixed_offset().into()).or_else(|_| active.try_set(column, at.into()))
}

impl<U, S> AuthStore<U::Model> for SeaOrmStore<U, S>
where
    U: UserEntity,
    S: SessionEntity,
    S::Column: Copy,
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
        set_timestamp(&mut active, S::CREATED_AT, new.created_at)?;
        set_timestamp(&mut active, S::EXPIRES_AT, new.expires_at)?;
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

pub fn create_sessions_table(users: impl IntoIden, users_id: impl IntoIden) -> TableCreateStatement {
    Table::create()
        .table(Alias::new("sessions"))
        .if_not_exists()
        .col(ColumnDef::new(Alias::new("id")).string().not_null().primary_key())
        .col(ColumnDef::new(Alias::new("user_id")).string().not_null())
        .col(ColumnDef::new(Alias::new("created_at")).timestamp_with_time_zone().not_null())
        .col(ColumnDef::new(Alias::new("expires_at")).timestamp_with_time_zone().not_null())
        .foreign_key(
            ForeignKey::create()
                .name("fk_sessions_user_id")
                .from(Alias::new("sessions"), Alias::new("user_id"))
                .to(users.into_iden(), users_id.into_iden())
                .on_delete(ForeignKeyAction::Cascade),
        )
        .to_owned()
}

pub fn create_sessions_user_id_index() -> IndexCreateStatement {
    Index::create().name("idx_sessions_user_id").table(Alias::new("sessions")).col(Alias::new("user_id")).to_owned()
}
