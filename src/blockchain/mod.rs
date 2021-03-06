use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};

use bitcoin::{Transaction, Txid};

use crate::database::{BatchDatabase, DatabaseUtils};
use crate::error::Error;

pub mod utils;

#[cfg(feature = "electrum")]
pub mod electrum;
#[cfg(feature = "electrum")]
pub use self::electrum::ElectrumBlockchain;

#[cfg(feature = "esplora")]
pub mod esplora;
#[cfg(feature = "esplora")]
pub use self::esplora::EsploraBlockchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    FullHistory,
    GetAnyTx,
}

pub trait Blockchain {
    fn is_online(&self) -> bool;

    fn offline() -> Self;
}

pub struct OfflineBlockchain;
impl Blockchain for OfflineBlockchain {
    fn offline() -> Self {
        OfflineBlockchain
    }

    fn is_online(&self) -> bool {
        false
    }
}

#[async_trait(?Send)]
pub trait OnlineBlockchain: Blockchain {
    async fn get_capabilities(&self) -> HashSet<Capability>;

    async fn setup<D: BatchDatabase + DatabaseUtils, P: Progress>(
        &mut self,
        stop_gap: Option<usize>,
        database: &mut D,
        progress_update: P,
    ) -> Result<(), Error>;
    async fn sync<D: BatchDatabase + DatabaseUtils, P: Progress>(
        &mut self,
        stop_gap: Option<usize>,
        database: &mut D,
        progress_update: P,
    ) -> Result<(), Error> {
        self.setup(stop_gap, database, progress_update).await
    }

    async fn get_tx(&mut self, txid: &Txid) -> Result<Option<Transaction>, Error>;
    async fn broadcast(&mut self, tx: &Transaction) -> Result<(), Error>;

    async fn get_height(&mut self) -> Result<usize, Error>;
}

pub type ProgressData = (f32, Option<String>);

pub trait Progress {
    fn update(&self, progress: f32, message: Option<String>) -> Result<(), Error>;
}

pub fn progress() -> (Sender<ProgressData>, Receiver<ProgressData>) {
    channel()
}

impl Progress for Sender<ProgressData> {
    fn update(&self, progress: f32, message: Option<String>) -> Result<(), Error> {
        if progress < 0.0 || progress > 100.0 {
            return Err(Error::InvalidProgressValue(progress));
        }

        self.send((progress, message))
            .map_err(|_| Error::ProgressUpdateError)
    }
}

pub struct NoopProgress;

pub fn noop_progress() -> NoopProgress {
    NoopProgress
}

impl Progress for NoopProgress {
    fn update(&self, _progress: f32, _message: Option<String>) -> Result<(), Error> {
        Ok(())
    }
}
