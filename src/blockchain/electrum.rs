use std::collections::HashSet;
use std::io::{Read, Write};

#[allow(unused_imports)]
use log::{debug, error, info, trace};

use bitcoin::{Script, Transaction, Txid};

use electrum_client::Client;

use self::utils::{ELSGetHistoryRes, ELSListUnspentRes, ElectrumLikeSync};
use super::*;
use crate::database::{BatchDatabase, DatabaseUtils};
use crate::error::Error;

pub struct ElectrumBlockchain<T: Read + Write + Send>(Option<Client<T>>);

impl<T: Read + Write + Send> std::convert::From<Client<T>> for ElectrumBlockchain<T> {
    fn from(client: Client<T>) -> Self {
        ElectrumBlockchain(Some(client))
    }
}

impl<T: Read + Write + Send> Blockchain for ElectrumBlockchain<T> {
    fn offline() -> Self {
        ElectrumBlockchain(None)
    }

    fn is_online(&self) -> bool {
        self.0.is_some()
    }
}

#[async_trait(?Send)]
impl<T: Read + Write + Send> OnlineBlockchain for ElectrumBlockchain<T> {
    fn get_capabilities(&self) -> HashSet<Capability> {
        vec![Capability::FullHistory, Capability::GetAnyTx]
            .into_iter()
            .collect()
    }

    fn setup<D: BatchDatabase + DatabaseUtils, P: Progress>(
        &mut self,
        stop_gap: Option<usize>,
        database: &mut D,
        progress_update: P,
    ) -> Result<(), Error> {
        self.0
            .as_mut()
            .ok_or(Error::OfflineClient)?
            .electrum_like_setup(stop_gap, database, progress_update)
    }

    fn get_tx(&mut self, txid: &Txid) -> Result<Option<Transaction>, Error> {
        Ok(self
            .0
            .as_mut()
            .ok_or(Error::OfflineClient)?
            .transaction_get(txid)
            .map(Option::Some)?)
    }

    fn broadcast(&mut self, tx: &Transaction) -> Result<(), Error> {
        Ok(self
            .0
            .as_mut()
            .ok_or(Error::OfflineClient)?
            .transaction_broadcast(tx)
            .map(|_| ())?)
    }

    fn get_height(&mut self) -> Result<usize, Error> {
        // TODO: unsubscribe when added to the client, or is there a better call to use here?

        Ok(self
            .0
            .as_mut()
            .ok_or(Error::OfflineClient)?
            .block_headers_subscribe()
            .map(|data| data.height)?)
    }
}

impl<T: Read + Write + Send> ElectrumLikeSync for Client<T> {
    fn els_batch_script_get_history<'s, I: IntoIterator<Item = &'s Script>>(
        &mut self,
        scripts: I,
    ) -> Result<Vec<Vec<ELSGetHistoryRes>>, Error> {
        self.batch_script_get_history(scripts)
            .map(|v| {
                v.into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(
                                |electrum_client::GetHistoryRes {
                                     height, tx_hash, ..
                                 }| ELSGetHistoryRes {
                                    height,
                                    tx_hash,
                                },
                            )
                            .collect()
                    })
                    .collect()
            })
            .map_err(Error::Electrum)
    }

    fn els_batch_script_list_unspent<'s, I: IntoIterator<Item = &'s Script>>(
        &mut self,
        scripts: I,
    ) -> Result<Vec<Vec<ELSListUnspentRes>>, Error> {
        self.batch_script_list_unspent(scripts)
            .map(|v| {
                v.into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(
                                |electrum_client::ListUnspentRes {
                                     height,
                                     tx_hash,
                                     tx_pos,
                                     ..
                                 }| ELSListUnspentRes {
                                    height,
                                    tx_hash,
                                    tx_pos,
                                },
                            )
                            .collect()
                    })
                    .collect()
            })
            .map_err(Error::Electrum)
    }

    fn els_transaction_get(&mut self, txid: &Txid) -> Result<Transaction, Error> {
        self.transaction_get(txid).map_err(Error::Electrum)
    }
}
