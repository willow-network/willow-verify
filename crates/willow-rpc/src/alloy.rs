//! alloy transports: every request a provider sends goes through the [`Verifier`], so every
//! response it returns was verified, or the provider gets the client's error.
//!
//! ```ignore
//! use alloy_provider::{Provider, ProviderBuilder};
//! use alloy_rpc_client::RpcClient;
//! use willow_rpc::{alloy::WillowTransport, Config};
//!
//! let transport = WillowTransport::new(Config::new(1).key(key))?;
//! let provider = ProviderBuilder::new().connect_client(RpcClient::new(transport, false));
//! ```
//!
//! [`WillowTransport`] is for alloy 2 (feature `alloy`); `v1::WillowTransport` for alloy 1
//! (feature `alloy-v1`). A batch the provider sends gets one verified response per item.

use crate::{Config, Error, Verifier};

macro_rules! transport {
    ($json_rpc:ident, $transport:ident) => {
        use super::*;
        use serde_json::value::RawValue;
        use std::borrow::Cow;
        use std::task::{Context, Poll};
        use $json_rpc::{
            ErrorPayload, RequestPacket, Response, ResponsePacket, ResponsePayload,
            SerializedRequest,
        };
        use $transport::{TransportError, TransportErrorKind, TransportFut};

        /// An alloy transport that verifies every response (a `tower::Service<RequestPacket>`).
        #[derive(Clone, Debug)]
        pub struct WillowTransport {
            verifier: Verifier,
        }

        impl WillowTransport {
            pub fn new(config: Config) -> Result<WillowTransport, Error> {
                Ok(WillowTransport {
                    verifier: Verifier::new(config)?,
                })
            }

            /// A transport over an existing client (shares its caches and light clients).
            pub fn from_verifier(verifier: Verifier) -> WillowTransport {
                WillowTransport { verifier }
            }

            pub fn verifier(&self) -> &Verifier {
                &self.verifier
            }
        }

        async fn one(v: &Verifier, req: SerializedRequest) -> Result<Response, TransportError> {
            let id = req.id().clone();
            let params: serde_json::Value = match req.params() {
                Some(p) => serde_json::from_str(p.get()).map_err(TransportError::ser_err)?,
                None => serde_json::Value::Null,
            };
            let payload = match v.request(req.method(), params).await {
                Ok(value) => ResponsePayload::Success(
                    RawValue::from_string(value.to_string()).map_err(TransportError::ser_err)?,
                ),
                Err(e) => {
                    let data = e
                        .to_json()
                        .get("data")
                        .map(|d| RawValue::from_string(d.to_string()))
                        .transpose()
                        .map_err(TransportError::ser_err)?;
                    ResponsePayload::Failure(ErrorPayload {
                        code: e.code(),
                        message: Cow::Owned(e.message()),
                        data,
                    })
                }
            };
            Ok(Response { id, payload })
        }

        impl tower::Service<RequestPacket> for WillowTransport {
            type Response = ResponsePacket;
            type Error = TransportError;
            type Future = TransportFut<'static>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, req: RequestPacket) -> Self::Future {
                let v = self.verifier.clone();
                Box::pin(async move {
                    match req {
                        RequestPacket::Single(r) => one(&v, r).await.map(ResponsePacket::Single),
                        RequestPacket::Batch(rs) => {
                            if rs.is_empty() {
                                return Err(TransportErrorKind::custom_str("an empty batch"));
                            }
                            let out =
                                futures::future::join_all(rs.into_iter().map(|r| one(&v, r))).await;
                            out.into_iter()
                                .collect::<Result<Vec<_>, _>>()
                                .map(ResponsePacket::Batch)
                        }
                    }
                })
            }
        }
    };
}

#[cfg(feature = "alloy")]
mod v2 {
    transport!(alloy_json_rpc, alloy_transport);
}
#[cfg(feature = "alloy")]
pub use v2::WillowTransport;

/// The same transport for alloy 1.
#[cfg(feature = "alloy-v1")]
pub mod v1 {
    transport!(alloy_json_rpc_v1, alloy_transport_v1);
}
