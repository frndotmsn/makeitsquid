use serde::Serialize;

#[derive(Serialize)]
pub struct IngameTemplateContext {
    pub template_image_uri: String,
    pub remaining_rerolls: i32,
    pub labels: Box<[i32]>,
}
