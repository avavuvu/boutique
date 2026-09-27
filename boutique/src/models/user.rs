use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub email: String,
    pub password: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::refresh_token::Entity")]
    RefreshToken,
}

impl Related<super::refresh_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RefreshToken.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
