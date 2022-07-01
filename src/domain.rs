use std::time::Duration;

use sqlx::{postgres::PgPool, FromRow};

pub enum DomainError {}
pub type DomainResult<T> = Result<T, DomainError>;

pub struct User {}

pub enum UserRepo {}

pub struct Org {
    channel_id: u64,
    name: String,
    descriptin: Option<String>,
    picture_url: Option<String>,
}

pub enum OrgRepo {
    Postgres(PgPool),
}

impl OrgRepo {
    pub async fn register_org(&self, org: &Org) -> DomainResult<()> {
        match self {
            Self::Postgres(_pool) => {}
        };
        unimplemented!()
    }
}

#[derive(Debug, PartialEq, FromRow)]
pub struct MenuItem {
    id: u64,
    title: String,
    icon: Option<String>,
    price_min: Option<u64>,
    price_max: Option<u64>,
    duration_min: Option<Duration>,
    duration_max: Option<Duration>,
}

pub enum MenuItemRepo {
    Postgres(PgPool),
}

impl MenuItemRepo {
    pub async fn find_by_parent(&self, parent_id: u64) -> DomainResult<Vec<MenuItem>> {
        match self {
            MenuItemRepo::Postgres(pool) => {}
        }

        todo!()
    }
}
