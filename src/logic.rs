/*
extern crate rand;

use althea_types::{Bytes32, EthAddress, EthPrivateKey, EthSignature};
// use crypto::Crypto;
use failure::{Error, SyncFailure};
use num256::Uint256;
use types::{Channel, ChannelStatus, Counterparty, NewChannelTx, UpdateTx, ChannelManager};
// use ethkey::{sign, Message, Secret};
use futures::{future, Future};
use web3::contract::{Contract, Options};
use web3::transports::http::Http;

#[derive(Debug, Fail)]
enum CallerServerError {
    #[fail(display = "Could not find counterparty")]
    CounterPartyNotFound {},
    #[fail(display = "Could not find channel")]
    ChannelNotFound {},
}

pub trait Storage {
    fn new_channel(&self, channel: ChannelManager) -> Result<(), Error>;
    fn save_channel(&self, channel: &ChannelManager) -> Result<(), Error>;
    fn save_update(&self, update: &UpdateTx) -> Result<(), Error>;
    fn get_counterparty_by_address(&self, &EthAddress) -> Result<Option<Counterparty>, Error>;
    fn get_channel_of_counterparty(&self, &Counterparty) -> Result<Option<ChannelManager>, Error>;
}

pub trait CounterpartyClient {
    fn make_payment(&self, &str, &UpdateTx) -> Box<Future<Item = EthSignature, Error = Error>>;
}

pub trait Crypto {
    fn hash_bytes(&self, &[&[u8]]) -> Bytes32;
    fn eth_sign(&self, &EthPrivateKey, &Bytes32) -> EthSignature;
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

pub struct CallerServer<'a> {
    pub crypto: &'a Crypto,
    pub counterpartyClient: &'a CounterpartyClient,
    pub storage: &'a Storage,
    pub contract: Contract<Http>,
    pub my_eth_address: EthAddress,
    pub challenge_length: Uint256,
}

fn wtf () {

}

impl<'a>
    CallerServer<'a>
{
    pub fn open_channel(
        &self,
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
                // .from_err()
                .map_err(SyncFailure::new)
                .from_err()
                .and_then(move |_foo| {
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
                    match self.storage.new_channel(channel) {
                        Err(err) => return Err(err),
                        _ => return Ok(()),
                    };
                }),
        )
    }

    // pub fn join_channel(
    //   &self,
    //   channel_Id: Bytes32,
    //   amount: Uint256
    // ) -> Result<(), Error> {
    //   // Call eth somehow
    //   Ok(())
    // }

    pub fn make_payment(
        self,
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

        let update_tx = channel.pay_counterparty(amount);

        self.storage.save_channel(&channel);
        self.storage.save_update(&update_tx);

        Box::new(
            self.counterpartyClient
                .make_payment(counterparty, &update_tx)
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

struct FakeStorage {}

impl Storage for FakeStorage {
    fn new_channel(&self, channel: Channel) -> Result<(), Error> {
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
    ) -> Result<Option<ChannelManager>, Error> {
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

struct FakeCrypto {}

impl Crypto for FakeCrypto {
    fn hash_bytes(&self, bytes: &[&[u8]]) -> Bytes32 {
        Bytes32([0; 32])
    }
    fn eth_sign(&self, key: &EthPrivateKey, input: &Bytes32) -> EthSignature {
        EthSignature([0; 65])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let callerServer = CallerServer {
            challenge_length: 0.into(),
            my_eth_address: EthAddress([0; 20]),
            storage: FakeStorage {},
            crypto: FakeCrypto {},
            counterpartyClient: FakeCounterpartyClient {},
        };

        callerServer.open_channel(0.into(), EthAddress([0; 20]));
        callerServer.make_payment("", EthAddress([0; 20]), 0.into());
    }
}
*/
