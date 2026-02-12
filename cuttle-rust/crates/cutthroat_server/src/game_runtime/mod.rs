mod commands;
mod task;
mod types;

pub(crate) use commands::GameCommand;
#[cfg(feature = "e2e-seed")]
pub(crate) use commands::{
    SeedGameFromTranscriptInput, SeedGameInput, SeedGameResult, SeedSeatInput,
};
pub(crate) use task::{create_game_for_user, create_rematch_for_user, game_sender};
#[cfg(feature = "e2e-seed")]
pub(crate) use task::{seed_game_from_tokenlog, seed_game_from_transcript};
pub(crate) use types::{
    GameAudience, GameEntry, GameStreamSubscription, GlobalRuntimeState, LobbySnapshotInternal,
    RuntimeError, SeatEntry,
};

pub(crate) const STATUS_LOBBY: i16 = 0;
pub(crate) const STATUS_STARTED: i16 = 1;
pub(crate) const STATUS_FINISHED: i16 = 2;
