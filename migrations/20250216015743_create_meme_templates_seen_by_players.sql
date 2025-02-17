-- Add migration script here
CREATE TABLE meme_templates_seen_by_players (
    meme_template_id INTEGER,
    player_id INTEGER,
    PRIMARY KEY (meme_template_id, player_id),
    FOREIGN KEY (meme_template_id)
        REFERENCES meme_templates(id)
        ON DELETE CASCADE,
    FOREIGN KEY (player_id)
        REFERENCES players(id)
        ON DELETE CASCADE
)
