// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::client::ClientHandshake;
use self::error::HandshakeError;
#[cfg(not(target_arch = "wasm32"))]
use self::gateway::GatewayHandshake;
pub use self::shared_key::{SharedKeySize, SharedKeys};
#[cfg(not(target_arch = "wasm32"))]
use coconut_interface::VerificationKey;
use crypto::asymmetric::identity;
use futures::{Sink, Stream};
use rand::{CryptoRng, RngCore};
use tungstenite::{Error as WsError, Message as WsMessage};

pub(crate) type WsItem = Result<WsMessage, WsError>;

mod client;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
mod gateway;
pub mod shared_key;
mod state;

// Note: the handshake is built on top of WebSocket, but in principle it shouldn't be too difficult
// to remove that restriction, by just changing Sink<WsMessage> and Stream<Item = WsMessage> into
// AsyncWrite and AsyncRead and slightly adjusting the implementation. But right now
// we do not need to worry about that.

pub async fn client_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    gateway_pubkey: identity::PublicKey,
    coconut_credential: coconut_interface::Credential,
) -> Result<SharedKeys, HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
{
    ClientHandshake::new(rng, ws_stream, identity, gateway_pubkey, coconut_credential).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn gateway_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    received_init_payload: Vec<u8>,
    verification_key: &VerificationKey,
) -> Result<SharedKeys, HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
{
    GatewayHandshake::new(
        rng,
        ws_stream,
        identity,
        received_init_payload,
        verification_key,
    )
    .await
}

/*

Messages exchanged:

CLIENT -> GATEWAY:
CLIENT_ID_KEY || G^x

GATEWAY -> CLIENT
G^y || AES(k, SIG(PRIV_G, G^y || G^x))

CLIENT -> GATEWAY
AES(k, SIG(PRIV_C, G^x || G^y))

GATEWAY -> CLIENT
DONE(status)

*/
