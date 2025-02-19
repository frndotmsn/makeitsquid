-- Add migration script here
CREATE TABLE meme_templates (
    id  INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uri TEXT NOT NULL
);

CREATE TABLE players (
    id   INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name TEXT NOT NULL,
    selected_meme_template_id INTEGER DEFAULT NULL,
    rerolls_left INTEGER NOT NULL DEFAULT 5,
    FOREIGN KEY (selected_meme_template_id)
        REFERENCES meme_templates(id)
        ON DELETE SET NULL
);
