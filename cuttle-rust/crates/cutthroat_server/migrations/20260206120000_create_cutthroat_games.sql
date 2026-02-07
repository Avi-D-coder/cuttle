CREATE TABLE cutthroat_games (
    id BIGSERIAL PRIMARY KEY,
    rust_game_id BIGINT NOT NULL UNIQUE,
    tokenlog TEXT NOT NULL,
    p0_username TEXT NOT NULL,
    p1_username TEXT NOT NULL,
    p2_username TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ NOT NULL,
    persisted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX cutthroat_games_id_desc_idx ON cutthroat_games (id DESC);
