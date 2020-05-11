use crate::model::Model;

#[derive(Deserialize, Serialize)]
pub struct MaskRequest {
    pub mask: Vec<u8>,
}

#[derive(Serialize)]
pub struct MaskResponse {
    pub key: String,
}

#[derive(Deserialize, Serialize)]
pub struct MaskedModelRequest {
    pub model: Vec<u8>,
}

#[derive(Serialize)]
pub struct MaskedModelResponse {
    pub key: String,
}

#[derive(Serialize)]
pub struct GlobalModelResponse<N>
where
    N: serde::Serialize,
{
    pub model: Model<N>,
}
