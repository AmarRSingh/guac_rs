extern crate rand;

use althea_types::{Bytes32, EthAddress, EthPrivateKey, EthSignature};
// use crypto::Crypto;
use failure::{Error, SyncFailure};
use num256::Uint256;
use types::{Channel, ChannelStatus, Counterparty, NewChannelTx, UpdateTx};
// use ethkey::{sign, Message, Secret};
use futures::{future, Future};
use web3::contract::{Contract, Options};
use web3::transports::http::Http;
use std::cell::RefCell;

#[derive(Debug, Fail)]
enum CallerServerError {
    #[fail(display = "Could not find counterparty")]
    CounterPartyNotFound {},
    #[fail(display = "Could not find channel")]
    ChannelNotFound {},
}

pub trait Storage {
    fn new_channel(&self, channel: &Channel) -> Result<(), Error>;
    fn save_channel(&self, channel: &Channel) -> Result<(), Error>;
    fn save_update(&self, update: &UpdateTx) -> Result<(), Error>;
    fn get_counterparty_by_address(&self, &EthAddress) -> Result<Option<Counterparty>, Error>;
    fn get_channel_of_counterparty(&self, &Counterparty) -> Result<Option<Channel>, Error>;
}

pub trait CounterpartyClient {
    fn make_payment(&self, &str, &UpdateTx) -> Box<Future<Item = EthSignature, Error = Error>>;
}


fn hash_bytes(bytes: &[&[u8]]) -> Bytes32 {
    Bytes32([0; 32])
}

fn eth_sign(key: &EthPrivateKey, input: &Bytes32) -> EthSignature {
    EthSignature([0; 65])
}


// pub struct CounterpartyServer {

// }

// impl CounterpartyServer {
//   pub fn make_payment(
//     &self,
//     update_tx: UpdateTx
//   ) -> Result<(), Error> {
//     Ok(())
//   }
// }

pub struct CallerServer<CPT: CounterpartyClient, STO: Storage> {
    pub counterpartyClient: CPT,
    pub storage: STO,
    pub contract: Contract<Http>,
    pub my_eth_address: EthAddress,
    pub challenge_length: Uint256,
}

impl<CPT: CounterpartyClient + 'static, STO: Storage + 'static>
    CallerServer<CPT, STO>
{
    pub fn open_channel(
        &'static self,
        amount: Uint256,
        their_eth_address: EthAddress,
    ) -> Box<Future<Item = (), Error = Error>> {
        let channel_id = Bytes32([0; 32]);
        Box::new(
            self.contract
                .call_with_confirmations(
                    "openChannel".into(),
                    (channel_id.0),
                    EthAddress([0; 20]).0.into(),
                    Options::with(|options| ()),
                    1u8.into(),
                )
                .map_err(SyncFailure::new)
                .from_err()
                .and_then(move |_| {
                    let channel = Channel {
                        channel_id,
                        address_a: self.my_eth_address,
                        address_b: their_eth_address,
                        channel_status: ChannelStatus::Open,
                        deposit_a: amount,
                        deposit_b: 0.into(),
                        challenge: self.challenge_length.clone(),
                        nonce: 0.into(),
                        close_time: 0.into(),
                        balance_a: 0.into(),
                        balance_b: 0.into(),
                        is_a: true,
                    };
                    match self.storage.new_channel(&channel) {
                        Err(err) => return Err(err),
                        _ => return Ok(()),
                    };
                }),
        )
    }

    pub fn join_channel(
      &self,
      channel_Id: Bytes32,
      amount: Uint256
    ) -> Result<(), Error> {
      // Call eth somehow
      Ok(())
    }

    pub fn make_payment(
        &'static self,
        their_url: &str,
        their_address: EthAddress,
        amount: Uint256,
    ) -> Box<Future<Item = (), Error = Error>> {
        let counterparty = match self.storage.get_counterparty_by_address(&their_address) {
            Ok(Some(counterparty)) => counterparty,
            Ok(None) => {
                return Box::new(future::err(Error::from(
                    CallerServerError::CounterPartyNotFound {},
                )))
            }
            Err(err) => return Box::new(future::err(err)),
        };

        let mut channel = match self.storage.get_channel_of_counterparty(&counterparty) {
            Ok(Some(channel)) => channel,
            Ok(None) => {
                return Box::new(future::err(Error::from(
                    CallerServerError::ChannelNotFound {},
                )))
            }
            Err(err) => return Box::new(future::err(err)),
        };

        let my_balance = channel.get_my_balance();
        let their_balance = channel.get_their_balance();

        channel.nonce = channel.nonce + 1;

        channel.set_my_balance(&(my_balance - amount.clone()));
        channel.set_their_balance(&(their_balance + amount));

        let mut update_tx = UpdateTx {
            channel_id: channel.channel_id.clone(),
            nonce: channel.nonce.clone() + 1,
            balance_a: channel.balance_a.clone(),
            balance_b: channel.balance_b.clone(),
            signature_a: None,
            signature_b: None,
        };

        let fingerprint = hash_bytes(&[
            update_tx.channel_id.as_ref(),
            &update_tx.nonce.to_bytes_le(),
            &update_tx.balance_a.to_bytes_le(),
            &update_tx.balance_b.to_bytes_le(),
        ]);

        let my_sig = eth_sign(&EthPrivateKey([0; 64]), &fingerprint);

        update_tx.set_my_signature(channel.is_a, &my_sig);

        self.storage.save_channel(&channel);
        self.storage.save_update(&update_tx);

        Box::new(
            self.counterpartyClient
                .make_payment(their_url, &update_tx)
                .from_err()
                .and_then(move |their_signature| {
                    update_tx.set_their_signature(channel.is_a, &their_signature);
                    match self.storage.save_channel(&channel) {
                        Err(err) => return Err(err),
                        _ => (),
                    };
                    match self.storage.save_update(&update_tx) {
                        Err(err) => return Err(err),
                        _ => (),
                    };

                    Ok(())
                }),
        )
    }

    // pub fn close_channel (&self, their_address: EthAddress) -> Box<Future<Item = (), Error = Error>> {

    // }
}

struct FakeStorage {

}

impl Storage for FakeStorage {
    fn new_channel(&self, channel: &Channel) -> Result<(), Error> {
        
        Ok(())
    }
    fn save_channel(&self, channel: &Channel) -> Result<(), Error> {
        Ok(())
    }
    fn save_update(&self, update: &UpdateTx) -> Result<(), Error> {
        Ok(())
    }
    fn get_counterparty_by_address(
        &self,
        eth_address: &EthAddress,
    ) -> Result<Option<Counterparty>, Error> {
        Ok(Some(Counterparty {
            address: *eth_address,
            url: String::from(""),
        }))
    }
    fn get_channel_of_counterparty(
        &self,
        eth_address: &Counterparty,
    ) -> Result<Option<Channel>, Error> {
        Ok(Some(Channel {
            channel_id: Bytes32([0; 32]),
            address_a: EthAddress([0; 20]),
            address_b: EthAddress([0; 20]),
            channel_status: ChannelStatus::Open,
            deposit_a: 0.into(),
            deposit_b: 0.into(),
            challenge: 0.into(),
            nonce: 0.into(),
            close_time: 0.into(),
            balance_a: 0.into(),
            balance_b: 0.into(),
            is_a: true,
        }))
    }
}

struct FakeCounterpartyClient {}

impl CounterpartyClient for FakeCounterpartyClient {
    fn make_payment(
        &self,
        url: &str,
        update: &UpdateTx,
    ) -> Box<Future<Item = EthSignature, Error = Error>> {
        Box::new(future::ok(EthSignature([0; 65])))
    }
}

mock_trait!(
    MockStorage,
    new_channel(Channel) -> Result<(), Error>,
    save_channel(Channel) -> Result<(), Error>,
    save_update(UpdateTx) -> Result<(), Error>,
    get_counterparty_by_address(EthAddress) -> Result<Option<Counterparty>, Error>,
    get_channel_of_counterparty(Counterparty) -> Result<Option<Channel>, Error>);


impl Storage for MockStorage {
    mock_method!(save_channel(&self, channel: &Channel) -> Result<(), Error>);
    mock_method!(new_channel(&self, channel: &Channel) -> Result<(), Error>);
    mock_method!(save_update(&self, update: &UpdateTx) -> Result<(), Error>);
    mock_method!(get_counterparty_by_address(&self, addr: &EthAddress) -> Result<Option<Counterparty>, Error>);
    mock_method!(get_channel_of_counterparty(&self, cpt: &Counterparty) -> Result<Option<Channel>, Error>);

}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn happy_path() {
//         let callerServer = CallerServer {
//             challenge_length: 0.into(),
//             my_eth_address: EthAddress([0; 20]),
//             storage: FakeStorage {},
//             crypto: FakeCrypto {},
//             counterpartyClient: FakeCounterpartyClient {},
//         };

//         callerServer.open_channel(0.into(), EthAddress([0; 20]));
//         callerServer.make_payment("", EthAddress([0; 20]), 0.into());
//     }
// }
