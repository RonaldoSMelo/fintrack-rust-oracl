use axum::{Json, Router, routing::get};
use serde::Deserialize;

use crate::{
    app::AppState, auth::admin::Admin, error::AppError, models::Asset, repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/assets",
        get(list_assets).post(create_asset).patch(update_asset),
    )
}

#[tracing::instrument(skip_all)]
async fn list_assets(repostiory: Repository) -> Result<Json<Vec<Asset>>, AppError> {
    let assets = repostiory.list_assets().await?;
    Ok(Json(assets))
}

#[derive(Deserialize)]
struct CreateAssetRequest {
    name: String,
    unit_value: f64,
}

#[tracing::instrument(skip_all)]
async fn create_asset(
    _: Admin,
    repostiory: Repository,
    Json(request): Json<CreateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    let new_asset = repostiory
        .create_asset(request.name, request.unit_value)
        .await?;

    Ok(Json(new_asset))
}

#[derive(Deserialize)]
struct UpdateAssetRequest {
    id: i64,
    name: Option<String>,
    unit_value: Option<f64>,
}

#[tracing::instrument(skip_all)]
async fn update_asset(
    _: Admin,
    repostiory: Repository,
    Json(request): Json<UpdateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    match repostiory
        .update_asset(request.id, request.name, request.unit_value)
        .await?
    {
        Some(updated_asset) => Ok(Json(updated_asset)),
        None => Err(AppError::AssetDoesNotExist),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use color_eyre::eyre::ensure;

    use super::*;
    use crate::{auth::user::UnauthenticatedUser, db::Database};

    #[tokio::test]
    #[ignore = "requires an Oracle schema with the migration applied"]
    async fn oracle_round_trip() -> color_eyre::Result<()> {
        dotenvy::dotenv().ok();
        let db = Database::from_env().await?;
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let name = format!("test-asset-{suffix}");
        let username = format!("test-user-{suffix}");
        let password = "test-password";

        let result = async {
            let Json(created) = create_asset(
                Admin,
                db.clone().into(),
                Json(CreateAssetRequest {
                    name: name.clone(),
                    unit_value: 10.0,
                }),
            )
            .await?;
            ensure!(created.name == name, "created asset name differs");
            ensure!(created.unit_value == 10.0, "created asset value differs");

            let Json(assets) = list_assets(db.clone().into()).await?;
            ensure!(
                assets
                    .iter()
                    .any(|asset| asset.id == created.id && asset.name == name)
            );

            let Json(updated) = update_asset(
                Admin,
                db.clone().into(),
                Json(UpdateAssetRequest {
                    id: created.id,
                    name: None,
                    unit_value: Some(20.0),
                }),
            )
            .await?;
            ensure!(updated.id == created.id, "updated asset ID differs");
            ensure!(updated.name == name, "partial update changed asset name");
            ensure!(updated.unit_value == 20.0, "updated asset value differs");

            let repo = Repository::from(db.clone());
            ensure!(repo.update_asset(-1, None, None).await?.is_none());
            let credentials = || UnauthenticatedUser::new(username.clone(), password.into());
            ensure!(matches!(
                credentials().authenticate(&repo).await,
                Err(AppError::UserDoesNotExist)
            ));
            let user = credentials().register(&repo).await?;
            ensure!(
                credentials().authenticate(&repo).await?.id() == user.id(),
                "authenticated user ID differs"
            );
            ensure!(matches!(
                credentials().register(&repo).await,
                Err(AppError::UsernameTaken)
            ));
            ensure!(!repo.register_purchase(user.id(), -1, 1.0, 10.0).await?);
            ensure!(
                repo.register_purchase(user.id(), created.id, 2.5, 12.0)
                    .await?
            );
            let purchases = repo.list_purchases(user.id()).await?;
            ensure!(purchases.len() == 1, "expected one recorded purchase");
            let purchase = &purchases[0];
            ensure!(purchase.asset.id == created.id);
            ensure!(purchase.quantity == 2.5 && purchase.unit_price == 12.0);
            ensure!(purchase.change() == 20.0, "unexpected position change");
            ensure!(repo.list_purchases(-1).await?.is_empty());
            Ok::<_, color_eyre::Report>(())
        }
        .await;

        db.run(move |conn| {
            conn.execute(
                "DELETE FROM purchases WHERE user_id = (SELECT id FROM users WHERE username = :1)",
                &[&username],
            )?;
            conn.execute("DELETE FROM assets WHERE name = :1", &[&name])?;
            conn.execute("DELETE FROM users WHERE username = :1", &[&username])?;
            conn.commit()
        })
        .await?;
        result
    }
}
