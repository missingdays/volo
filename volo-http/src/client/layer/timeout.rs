use motore::{layer::Layer, service::Service};
use volo::context::Context;

use crate::{context::client::Config, error::ClientError};

/// [`Layer`] for applying timeout from [`Config`].
///
/// This layer will be applied by default when using [`ClientBuilder::build`], without this layer,
/// timeout from [`Client`] or [`CallOpt`] will not work.
///
/// [`Client`]: crate::client::Client
/// [`ClientBuilder::build`]: crate::client::ClientBuilder::build
/// [`CallOpt`]: crate::client::CallOpt
#[derive(Clone, Debug, Default)]
pub struct Timeout;

impl<S> Layer<S> for Timeout {
    type Service = TimeoutService<S>;

    fn layer(self, inner: S) -> Self::Service {
        TimeoutService { inner }
    }
}

/// The [`Service`] generated by [`Timeout`].
///
/// See [`Timeout`] for more details.
pub struct TimeoutService<S> {
    inner: S,
}

impl<Cx, Req, S> Service<Cx, Req> for TimeoutService<S>
where
    Cx: Context<Config = Config> + Send,
    Req: Send,
    S: Service<Cx, Req, Error = ClientError> + Send + Sync,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&self, cx: &mut Cx, req: Req) -> Result<Self::Response, Self::Error> {
        let timeout = cx.rpc_info().config().timeout().cloned();
        let fut = self.inner.call(cx, req);

        if let Some(duration) = timeout {
            let sleep = tokio::time::sleep(duration);

            tokio::select! {
                res = fut => res,
                _ = sleep => {
                    tracing::error!("[Volo-HTTP] request timeout");
                    Err(crate::error::client::timeout().with_endpoint(cx.rpc_info().callee()))
                }
            }
        } else {
            fut.await
        }
    }
}
