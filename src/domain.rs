use std::{num::ParseIntError, str::FromStr};

use anyhow::Result;
use chrono::Duration;
use sqlx::{postgres::PgPool, query, query_as, FromRow, Type};

use serde::{Deserialize, Serialize};

pub struct User {}

pub enum UserRepo {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Type)]
#[sqlx(transparent)]
pub struct OrgId(pub i64);

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
    pub async fn register_org(&self, org: &Org) -> Result<()> {
        match self {
            Self::Postgres(_pool) => {}
        };
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum IntervalValue<T: std::fmt::Debug> {
    Value(T),
    Interval(T, T),
}

impl<T: std::fmt::Debug> From<(Option<T>, Option<T>)> for IntervalValue<T> {
    fn from((value1, value2): (Option<T>, Option<T>)) -> Self {
        if value1.is_some() && value2.is_some() {
            IntervalValue::Interval(value1.unwrap(), value2.unwrap())
        } else if let Some(value) = value1 {
            IntervalValue::Value(value)
        } else if let Some(value) = value2 {
            IntervalValue::Value(value)
        } else {
            unreachable!("Both interval values are not set")
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Type, Serialize, Deserialize)]
#[sqlx(transparent)]
pub struct MenuItemId(pub i64);

impl ToString for MenuItemId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl FromStr for MenuItemId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        i64::from_str(s).map(|v| Self(v))
    }
}

#[derive(Debug)]
pub struct MenuItem {
    pub id: MenuItemId,
    pub title: String,
    pub icon: Option<String>,
    pub price: IntervalValue<i32>,
    pub duration: IntervalValue<Duration>,
}

impl From<MenuItemRow> for MenuItem {
    fn from(
        MenuItemRow {
            id,
            title,
            icon,
            price_min,
            price_max,
            duration_min,
            duration_max,
        }: MenuItemRow,
    ) -> Self {
        Self {
            id: MenuItemId(id),
            title,
            icon,
            price: (price_min, price_max).into(),
            duration: (
                duration_min.map(Duration::minutes),
                duration_max.map(Duration::minutes),
            )
                .into(),
        }
    }
}

#[derive(Debug, FromRow)]
struct MenuItemRow {
    id: i64,
    title: String,
    icon: Option<String>,
    price_min: Option<i32>,
    price_max: Option<i32>,
    duration_min: Option<i64>,
    duration_max: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum MenuItemRepo {
    Postgres(PgPool),
}

impl MenuItemRepo {
    pub async fn find_by_org(
        &self,
        org_id: OrgId,
        parent_id: Option<MenuItemId>,
    ) -> Result<Vec<MenuItem>> {
        match self {
            MenuItemRepo::Postgres(pool) => {
                let rows = if let Some(parent_id) = parent_id {
                    query_as!(
                        MenuItemRow,
                        r#"
                    SELECT id, title, icon, price_min,
                    price_max, duration_min, duration_max
                    FROM menu_item WHERE org_id = $1 AND parent_id = $2
                    "#,
                        org_id.0,
                        parent_id.0,
                    )
                    .fetch_all(pool)
                    .await?
                } else {
                    query_as!(
                        MenuItemRow,
                        r#"
                    SELECT id, title, icon, price_min,
                    price_max, duration_min, duration_max
                    FROM menu_item WHERE parent_id IS NULL AND org_id = $1
                    "#,
                        org_id.0
                    )
                    .fetch_all(pool)
                    .await?
                };

                Ok(rows.into_iter().map(MenuItem::from).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::{clients::Cli, images::postgres::Postgres};

    use crate::db::migration;

    #[tokio::test]
    async fn test_menu() -> anyhow::Result<()> {
        let docker = Cli::default();
        let pg_node = docker.run(Postgres::default());

        let db_url = format!(
            "postgres://postgres@localhost:{}",
            pg_node.get_host_port_ipv4(5432)
        );

        let pool = sqlx::PgPool::connect(&db_url).await?;
        migration::apply_only(&pool, &vec!["menu"]).await?;

        Ok(())
    }
}
