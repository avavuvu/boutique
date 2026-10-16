use sea_orm::sea_query::{
    Alias, ColumnDef, ForeignKey, ForeignKeyAction, Index, IndexCreateStatement, IntoIden, Table, TableCreateStatement,
};

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
    Index::create()
        .name("idx_sessions_user_id")
        .table(Alias::new("sessions"))
        .col(Alias::new("user_id"))
        .to_owned()
}
