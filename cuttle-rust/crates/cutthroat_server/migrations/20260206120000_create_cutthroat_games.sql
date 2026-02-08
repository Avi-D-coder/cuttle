CREATE TABLE cutthroat_games (
    id BIGSERIAL PRIMARY KEY,
    rust_game_id BIGINT NOT NULL UNIQUE,
    tokenlog TEXT NOT NULL,
    p0_user_id BIGINT NOT NULL,
    p1_user_id BIGINT NOT NULL,
    p2_user_id BIGINT NOT NULL,
    CONSTRAINT cutthroat_games_p0_user_fk FOREIGN KEY (p0_user_id) REFERENCES "user"(id),
    CONSTRAINT cutthroat_games_p1_user_fk FOREIGN KEY (p1_user_id) REFERENCES "user"(id),
    CONSTRAINT cutthroat_games_p2_user_fk FOREIGN KEY (p2_user_id) REFERENCES "user"(id),
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ NOT NULL,
    persisted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX cutthroat_games_id_desc_idx ON cutthroat_games (id DESC);
CREATE INDEX cutthroat_games_finished_at_desc_idx ON cutthroat_games (finished_at DESC, rust_game_id DESC);
CREATE INDEX cutthroat_games_p0_user_time_idx ON cutthroat_games (p0_user_id, finished_at DESC, rust_game_id DESC);
CREATE INDEX cutthroat_games_p1_user_time_idx ON cutthroat_games (p1_user_id, finished_at DESC, rust_game_id DESC);
CREATE INDEX cutthroat_games_p2_user_time_idx ON cutthroat_games (p2_user_id, finished_at DESC, rust_game_id DESC);
CREATE INDEX cutthroat_games_started_at_desc_idx ON cutthroat_games (started_at DESC);
