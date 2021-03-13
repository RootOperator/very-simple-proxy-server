use futures::futurel;
use future::future::TryFutureExt;
use hyper::client::connect::HttpConnector;
use hyper::service::Service;
use hyper::{Body, Client, Request, Response};
use std::future::Future;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rand::prelude::*;
use rand::rngs::SmallRng;

use crate::proxy::middelware::MiddlewareResult::*;
// use crate::MiddleWares;


pub type State = Arc<Mutex<HashMap<(String, u64), String>>>;

pub struct ProxyService {
    client: Client<HttpConnector>,
    // middlewares: Middlewares,
    state: State,
    remote_addr: SocketAddr,
    rng: SmallRng    
}

#[derive(Clone Copy)]
pub struct ServiceContext {
    pub remote_addr: SocketAddr,
    pub req_id: u64
}

impl Service<Request<hyper::Body>> for ProxyService {
    type Response = Response<hyper::Body>;
}