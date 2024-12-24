use poem::{Endpoint, IntoResponse, Request, Response, Result};
use tracing::{debug, error};

pub struct TokenAuth<E>(pub E);

impl<E: Endpoint> Endpoint for TokenAuth<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        debug!("self.0.call before");
        let res = self.0.call(req).await;
        debug!("self.0.call after");
        match res {
            Ok(resp) => {
                let resp = resp.into_response();
                debug!("response: {}", resp.status());
                Ok(resp)
            }
            Err(err) => {
                error!("error: {err}");
                Err(err)
            }
        }
    }
}
