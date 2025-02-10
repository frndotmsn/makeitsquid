use crate::labeled_meme::LabeledMeme;

pub struct UnlabeledMeme {
    image_uri: String,
    no_label_slots: i32,
    label_positions: Box<[i32]>
}

impl UnlabeledMeme {
    pub fn label(&self, labels: &[&str]) -> Result<LabeledMeme, anyhow::Error>
    {
        unimplemented!()
    }
}