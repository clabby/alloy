use crate::{BatchRequest, ClientBuilder, RpcCall};
use alloy_json_rpc::{Id, Request, RpcParam, RpcReturn};
use alloy_transport::{BoxTransport, Transport, TransportConnect, TransportResult};
use alloy_transport_http::Http;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::{layer::util::Identity, ServiceBuilder};

/// A JSON-RPC client.
///
/// This struct manages a [`Transport`] and a request ID counter. It is used to
/// build [`RpcCall`] and [`BatchRequest`] objects. The client delegates
/// transport access to the calls.
///
/// ### Note
///
/// IDs are allocated sequentially, starting at 0. IDs are reserved via
/// [`RpcClient::next_id`]. Note that allocated IDs may not be used. There is
/// no guarantee that a prepared [`RpcCall`] will be sent, or that a sent call
/// will receive a response.
#[derive(Debug)]
pub struct RpcClient<T> {
    /// The underlying transport.
    pub(crate) transport: T,
    /// `true` if the transport is local.
    pub(crate) is_local: bool,
    /// The next request ID to use.
    pub(crate) id: AtomicU64,
}

impl RpcClient<Identity> {
    /// Create a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder<Identity> {
        ClientBuilder { builder: ServiceBuilder::new() }
    }
}

impl<T> RpcClient<T> {
    /// Create a new [`RpcClient`] with the given transport.
    pub const fn new(t: T, is_local: bool) -> Self {
        Self { transport: t, is_local, id: AtomicU64::new(0) }
    }

    /// Connect to a transport via a [`TransportConnect`] implementor.
    pub async fn connect<C>(connect: C) -> TransportResult<Self>
    where
        T: Transport,
        C: TransportConnect<Transport = T>,
    {
        ClientBuilder::default().connect(connect).await
    }

    /// Build a `JsonRpcRequest` with the given method and params.
    ///
    /// This function reserves an ID for the request, however the request
    /// is not sent. To send a request, use [`RpcClient::prepare`] and await
    /// the returned [`RpcCall`].
    pub fn make_request<Params: RpcParam>(
        &self,
        method: &'static str,
        params: Params,
    ) -> Request<Params> {
        Request::new(method, self.next_id(), params)
    }

    /// `true` if the client believes the transport is local.
    ///
    /// This can be used to optimize remote API usage, or to change program
    /// behavior on local endpoints. When the client is instantiated by parsing
    /// a URL or other external input, this value is set on a best-efforts
    /// basis and may be incorrect.
    #[inline]
    pub const fn is_local(&self) -> bool {
        self.is_local
    }

    /// Set the `is_local` flag.
    pub fn set_local(&mut self, is_local: bool) {
        self.is_local = is_local;
    }

    /// Reserve a request ID value. This is used to generate request IDs.
    #[inline]
    fn increment_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    /// Reserve a request ID u64.
    #[inline]
    pub fn next_id(&self) -> Id {
        Id::Number(self.increment_id())
    }
}

impl<T> RpcClient<T>
where
    T: Transport + Clone,
{
    /// Prepare an [`RpcCall`].
    ///
    /// This function reserves an ID for the request, however the request
    /// is not sent. To send a request, await the returned [`RpcCall`].
    ///
    /// ### Note:
    ///
    /// Serialization is done lazily. It will not be performed until the call
    /// is awaited. This means that if a serializer error occurs, it will not
    /// be caught until the call is awaited.
    pub fn prepare<Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Params,
    ) -> RpcCall<T, Params, Resp> {
        let request = self.make_request(method, params);
        RpcCall::new(request, self.transport.clone())
    }

    /// Type erase the service in the transport, allowing it to be used in a
    /// generic context.
    ///
    /// ## Note:
    ///
    /// This is for abstracting over `RpcClient<T>` for multiple `T` by
    /// erasing each type. E.g. if you have `RpcClient<Http>` and
    /// `RpcClient<Ws>` you can put both into a `Vec<RpcClient<BoxTransport>>`.
    #[inline]
    pub fn boxed(self) -> RpcClient<BoxTransport> {
        RpcClient { transport: self.transport.boxed(), is_local: self.is_local, id: self.id }
    }
}

#[cfg(feature = "pubsub")]
mod pubsub_impl {
    use super::*;
    use alloy_pubsub::{PubSubConnect, PubSubFrontend, RawSubscription, Subscription};

    impl RpcClient<PubSubFrontend> {
        /// Get a [`RawSubscription`] for the given subscription ID.
        pub async fn get_raw_subscription(&self, id: alloy_primitives::U256) -> RawSubscription {
            self.transport.get_subscription(id).await.unwrap()
        }

        /// Get a [`Subscription`] for the given subscription ID.
        pub async fn get_subscription<T: serde::de::DeserializeOwned>(
            &self,
            id: alloy_primitives::U256,
        ) -> Subscription<T> {
            Subscription::from(self.get_raw_subscription(id).await)
        }

        /// Connect to a transport via a [`PubSubConnect`] implementor.
        pub async fn connect_pubsub<C>(connect: C) -> TransportResult<RpcClient<PubSubFrontend>>
        where
            C: PubSubConnect,
        {
            ClientBuilder::default().pubsub(connect).await
        }

        /// Get the currently configured channel size. This is the number of items
        /// to buffer in new subscription channels. Defaults to 16. See
        /// [`tokio::sync::broadcast`] for a description of relevant
        /// behavior.
        ///
        /// [`tokio::sync::broadcast`]: https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html
        pub const fn channel_size(&self) -> usize {
            self.transport.channel_size()
        }

        /// Set the channel size. This is the number of items to buffer in new
        /// subscription channels. Defaults to 16. See
        /// [`tokio::sync::broadcast`] for a description of relevant
        /// behavior.
        ///
        /// [`tokio::sync::broadcast`]: https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html
        pub fn set_channel_size(&mut self, size: usize) {
            self.transport.set_channel_size(size);
        }
    }
}

impl<T> RpcClient<Http<T>> {
    /// Create a new [`BatchRequest`] builder.
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<'_, Http<T>> {
        BatchRequest::new(self)
    }
}
