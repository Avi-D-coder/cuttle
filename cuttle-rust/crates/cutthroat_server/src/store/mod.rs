mod commands;
mod task;
mod types;

pub(crate) use commands::Command;
pub(crate) use task::store_task;
pub(crate) use types::{GameEntry, SeatEntry, Store, StoreError};

pub(crate) const STATUS_LOBBY: i16 = 0;
pub(crate) const STATUS_STARTED: i16 = 1;
pub(crate) const STATUS_FINISHED: i16 = 2;
