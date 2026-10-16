use sea_orm::{
    IdenStatic,
    sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Index, IndexCreateStatement, Table, TableCreateStatement},
};

use super::{SessionEntity, UserEntity};

pub fn create_sessions_table<U: UserEntity, S: SessionEntity>() -> TableCreateStatement {
    let sessions = S::default();
    Table::create()
        .table(sessions)
        .if_not_exists()
        .col(ColumnDef::new(S::TOKEN_HASH).string().not_null().primary_key())
        .col(ColumnDef::new(S::USER_ID).string().not_null())
        .col(ColumnDef::new(S::CREATED_AT).timestamp_with_time_zone().not_null())
        .col(ColumnDef::new(S::EXPIRES_AT).timestamp_with_time_zone().not_null())
        .foreign_key(
            ForeignKey::create()
                .name(format!("fk_{}_{}", sessions.table_name(), S::USER_ID.as_str()))
                .from(sessions, S::USER_ID)
                .to(U::default(), U::ID)
                .on_delete(ForeignKeyAction::Cascade),
        )
        .to_owned()
}

pub fn create_sessions_user_id_index<S: SessionEntity>() -> IndexCreateStatement {
    let sessions = S::default();
    Index::create()
        .name(format!("idx_{}_{}", sessions.table_name(), S::USER_ID.as_str()))
        .table(sessions)
        .col(S::USER_ID)
        .to_owned()
}
