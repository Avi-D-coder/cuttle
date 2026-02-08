CREATE TABLE cutthroat_games (
    id BIGSERIAL PRIMARY KEY,
    rust_game_id BIGINT NOT NULL UNIQUE,
    tokenlog TEXT NOT NULL,
    -- Semantically, an FK to "user"(id) would be correct.
    -- We intentionally avoid it to preserve looser service independence and to avoid Sails
    -- auto-migration drops/recreates of "user" breaking cutthroat persistence.
    -- Note: full independence would also require removing the SQL join on "user" in the
    -- 3P history query. We would return only user IDs to the frontend and have it call
    -- a JS route to resolve usernames. We do not do that currently.
    p0_user_id BIGINT NOT NULL,
    p1_user_id BIGINT NOT NULL,
    p2_user_id BIGINT NOT NULL,
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
