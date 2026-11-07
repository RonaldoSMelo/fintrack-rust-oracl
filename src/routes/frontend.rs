use askama::Template;
use axum::{
    Form, Router,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use serde::Deserialize;

use crate::{
    app::AppState,
    auth::user::{UnauthenticatedUser, User},
    error::AppError,
    models::{Asset, Purchase},
    repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/login", get(login_page).post(login))
        .route("/assets", get(dashboard))
        .route(
            "/assets/purchase",
            get(purchase_page).post(register_purchase),
        )
        .route("/logout", post(logout))
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

async fn login_page() -> Result<Html<String>, AppError> {
    let html = LoginPage.render()?;
    Ok(Html(html))
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login(
    repository: Repository,
    jar: CookieJar,
    Form(request): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let unauth_user = UnauthenticatedUser::new(request.username, request.password);
    let user = match unauth_user.authenticate(&repository).await {
        Ok(user) => user,
        Err(AppError::UserDoesNotExist) => unauth_user.register(&repository).await?,
        Err(other_err) => return Err(other_err),
    };

    let token = user.auth_token()?;
    let cookie = Cookie::build(("token", token))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true);

    Ok((jar.add(cookie), Redirect::to("/assets")))
}

async fn index(user: Option<User>) -> Redirect {
    if user.is_some() {
        Redirect::to("/assets")
    } else {
        Redirect::to("/login")
    }
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    (
        jar.remove(Cookie::build("token").path("/")),
        Redirect::to("/login"),
    )
}

#[derive(Template)]
#[template(path = "assets.html")]
struct Dashboard<'a> {
    username: &'a str,
    positions: Vec<Position<'a>>,
}

struct Position<'a> {
    asset: &'a Asset,
    purchases: &'a [Purchase],
    quantity: f64,
    change: f64,
}

fn money(value: f64) -> String {
    format!("R$ {value:.2}").replace('.', ",")
}

fn signed_money(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 {
        money(0.0)
    } else {
        let sign = if rounded > 0.0 { "+" } else { "−" };
        format!("{sign} {}", money(rounded.abs()))
    }
}

fn change_class(value: f64) -> &'static str {
    if value >= 0.005 {
        "gain"
    } else if value <= -0.005 {
        "loss"
    } else {
        ""
    }
}

fn quantity_text(value: f64) -> String {
    format!("{value:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

async fn dashboard(user: Option<User>, repository: Repository) -> Result<Response, AppError> {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let purchases = repository.list_purchases(user.id()).await?;
    let positions = purchases
        .chunk_by(|a, b| a.asset.id == b.asset.id)
        .map(|purchases| Position {
            asset: &purchases[0].asset,
            quantity: purchases.iter().map(|purchase| purchase.quantity).sum(),
            change: purchases.iter().map(Purchase::change).sum(),
            purchases,
        })
        .collect();
    let page = Dashboard {
        username: user.username(),
        positions,
    };
    Ok(Html(page.render()?).into_response())
}

#[derive(Template)]
#[template(path = "purchase.html")]
struct PurchasePage {
    assets: Vec<Asset>,
    error: Option<&'static str>,
}

#[derive(Deserialize)]
struct PurchaseForm {
    asset_id: i64,
    quantity: f64,
    unit_price: f64,
}

async fn purchase_page(user: Option<User>, repository: Repository) -> Result<Response, AppError> {
    if user.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }
    render_purchase_page(&repository, StatusCode::OK, None).await
}

async fn render_purchase_page(
    repository: &Repository,
    status: StatusCode,
    error: Option<&'static str>,
) -> Result<Response, AppError> {
    let page = PurchasePage {
        assets: repository.list_assets().await?,
        error,
    };
    Ok((status, Html(page.render()?)).into_response())
}

async fn register_purchase(
    user: Option<User>,
    repository: Repository,
    Form(form): Form<PurchaseForm>,
) -> Result<Response, AppError> {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let valid = |value: f64| (0.00000001..=9_999_999_999.0).contains(&value);
    if !valid(form.quantity) || !valid(form.unit_price) {
        return render_purchase_page(
            &repository,
            StatusCode::BAD_REQUEST,
            Some("Informe uma quantidade e um preço positivos dentro do limite do formulário."),
        )
        .await;
    }
    if !repository
        .register_purchase(user.id(), form.asset_id, form.quantity, form.unit_price)
        .await?
    {
        return render_purchase_page(
            &repository,
            StatusCode::NOT_FOUND,
            Some("O ativo selecionado não está disponível."),
        )
        .await;
    }
    Ok(Redirect::to("/assets").into_response())
}
