use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Asset {
    pub id: i64,
    pub name: String,
    pub unit_value: f64,
}

pub struct UserRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

pub struct Purchase {
    pub asset: Asset,
    pub quantity: f64,
    pub unit_price: f64,
    pub purchased_at: String,
}

impl Purchase {
    pub fn change(&self) -> f64 {
        (self.asset.unit_value - self.unit_price) * self.quantity
    }
}
