use std::convert::Infallible;

use axum::extract::FromRequestParts;
use oracledb::{Error, ErrorKind, Row};

use crate::{
    app::AppState,
    db::{Database, DatabaseError},
    models::{Asset, UserRecord},
};

pub struct Repository {
    db: Database,
}

impl Repository {
    pub async fn list_assets(&self) -> Result<Vec<Asset>, DatabaseError> {
        self.db
            .run(|conn| {
                conn.query("SELECT id, name, unit_value FROM assets ORDER BY id", &[])?
                    .map(|row| read_asset(row?))
                    .collect()
            })
            .await
    }

    pub async fn create_asset(
        &self,
        name: String,
        unit_value: f64,
    ) -> Result<Asset, DatabaseError> {
        self.db
            .run(move |conn| {
                let id: i64 = conn
                    .query_row("SELECT assets_seq.NEXTVAL FROM dual", &[])?
                    .get(0)?;
                conn.execute(
                    "INSERT INTO assets (id, name, unit_value) VALUES (:1, :2, :3)",
                    &[&id, &name, &unit_value],
                )?;
                conn.commit()?;
                Ok(Asset {
                    id,
                    name,
                    unit_value,
                })
            })
            .await
    }

    pub async fn update_asset(
        &self,
        asset_id: i64,
        name: Option<String>,
        unit_value: Option<f64>,
    ) -> Result<Option<Asset>, DatabaseError> {
        self.db
            .run(move |conn| {
                let result = conn.execute(
                    "UPDATE assets
                 SET name = COALESCE(:1, name), unit_value = COALESCE(:2, unit_value)
                 WHERE id = :3",
                    &[&name, &unit_value, &asset_id],
                )?;
                if result.rows_affected() == 0 {
                    conn.rollback()?;
                    return Ok(None);
                }
                let asset = read_asset(conn.query_row(
                    "SELECT id, name, unit_value FROM assets WHERE id = :1",
                    &[&asset_id],
                )?)?;
                conn.commit()?;
                Ok(Some(asset))
            })
            .await
    }

    pub async fn add_user(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, DatabaseError> {
        let username = username.to_owned();
        let password_hash = password_hash.to_owned();
        self.db
            .run(move |conn| {
                let id: i64 = conn
                    .query_row("SELECT users_seq.NEXTVAL FROM dual", &[])?
                    .get(0)?;
                conn.execute(
                    "INSERT INTO users (id, username, password_hash) VALUES (:1, :2, :3)",
                    &[&id, &username, &password_hash],
                )?;
                conn.commit()?;
                Ok(UserRecord {
                    id,
                    username,
                    password_hash,
                })
            })
            .await
    }

    pub async fn get_user_by_name(
        &self,
        username: &str,
    ) -> Result<Option<UserRecord>, DatabaseError> {
        let username = username.to_owned();
        self.db
            .run(move |conn| {
                let row = match conn.query_row(
                    "SELECT id, username, password_hash FROM users WHERE username = :1",
                    &[&username],
                ) {
                    Ok(row) => row,
                    Err(error) if matches!(error.kind(), ErrorKind::NoDataFound) => {
                        return Ok(None);
                    }
                    Err(error) => return Err(error),
                };
                Ok(Some(UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                }))
            })
            .await
    }
}

fn read_asset(row: Row) -> Result<Asset, Error> {
    Ok(Asset {
        id: row.get(0)?,
        name: row.get(1)?,
        unit_value: row.get(2)?,
    })
}

impl FromRequestParts<AppState> for Repository {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self {
            db: state.db.clone(),
        })
    }
}

#[cfg(test)]
impl From<Database> for Repository {
    fn from(db: Database) -> Self {
        Self { db }
    }
}
