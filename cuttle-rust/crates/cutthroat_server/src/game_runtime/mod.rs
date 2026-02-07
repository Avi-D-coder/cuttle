mod commands;
mod task;
mod types;

pub(crate) use commands::Command;
pub(crate) use task::runtime_task;
pub(crate) use types::{
    GameAudience, GameEntry, GameRuntime, GameStreamSubscription, LobbySnapshotInternal,
    RuntimeError, SeatEntry,
};
#[cfg(feature = "e2e-seed")]
pub(crate) use commands::{SeedGameInput, SeedGameResult, SeedSeatInput};

pub(crate) const STATUS_LOBBY: i16 = 0;
pub(crate) const STATUS_STARTED: i16 = 1;
pub(crate) const STATUS_FINISHED: i16 = 2;
