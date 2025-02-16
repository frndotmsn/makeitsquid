#[derive(Clone)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub selected_meme_template_id: Option<String>,
    pub seen_meme_templates: Vec<String>,
}

impl Player {
    pub fn new_guest(name: String) -> Player {
        Player {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            selected_meme_template_id: None,
            seen_meme_templates: vec![],
        }
    }
}
