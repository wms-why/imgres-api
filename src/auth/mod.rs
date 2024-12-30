pub mod token_auth;

use poem::{Endpoint, Middleware};

pub struct Auth;

impl<E: Endpoint> Middleware<E> for Auth {
    type Output = token_auth::TokenAuth<E>;

    fn transform(&self, ep: E) -> Self::Output {
        token_auth::TokenAuth(ep)
    }
}
