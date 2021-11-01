#[derive(serde::Deserialize, Debug)]
pub struct Transaction {
    pub r#type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f32>,
}
