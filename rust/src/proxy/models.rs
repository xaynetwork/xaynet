use crate::model::Model;
#[derive(Serialize)]
pub struct MaskResponse {
    pub key: Option<String>,
}

#[derive(Serialize)]
pub struct MaskedModelResponse {
    pub key: Option<String>,
}

#[derive(Serialize)]
pub struct GlobalModelResponse<N>
where
    N: serde::Serialize,
{
    pub global_model: Option<Model<N>>,
}
