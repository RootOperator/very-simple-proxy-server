use futures::future;
use futures::future::TryFutureExt;
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

use crate::proxy::middleware::MiddlewareResult::*;
use crate::Middlewares;

pub type State = Arc<Mutex<HashMap<(String, u64), String>>>;

pub struct ProxyService {
    client: Client<HttpConnector>,
    middlewares: Middlewares,
    state: State,
    remote_addr: SocketAddr,
    rng: SmallRng,
}

#[derive(Clone, Copy)]
pub struct ServiceContext {
    pub remote_addr: SocketAddr,
    pub req_id: u64,
}

impl Service<Request<hyper::Body>> for ProxyService {
    type Response = Response<hyper::Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.client.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending
        }
    }

    fn call(&mut self, req: Request<hyper::Body>) -> Self::Future {
        self.clear_state();
        let (parts, body) = req.into_parts();
        let mut req = Request::from_parts(parts, body);

        let mws_failure = Arc::clone(&self.middlewares);
        let mws_success = Arc::clone(&self.middlewares);
        let mws_after_success = Arc::clone(&self.middlewares);
        let mws_after_failure = Arc::clone(&self.middlewares);
        let state_failure = Arc::clone(&self.state);
        let state_success = Arc::clone(&self.state);
        let state_after_success = Arc::clone(&self.state);
        let state_after_failure = Arc::clone(&self.state);

        let req_id = self.rng.next_u64();

        let context = ServiceContext {
            req_id,
            remote_addr: self.remote_addr
        };

        let mut before_res: Option<Response<Body>> = None;
        for mw in self.middlewares.lock().unwrap().iter_mut() {
            if let Some(res) = match mw.before_request(&mut req, &context, &self.state) {
                Err(err) => Some(Response::from(err)),
                Ok(RespondWith(response)) => Some(response),
                Ok(Next) => None
            } {
                before_res = Some(res);
                break;
            }
        }

        if let Some(res) = before_res {
            return Box::pin(future::ok(self.early_response(&context, res, &self.state)));
        }

        let res = self
        .client
        .request(req)
        .map_err(move |err| {
            for mw in mws_failure.lock().unwrap().iter_mut() {
                if let Err(err) = mw.request_failure(&err, &context, &state_failure) {
                    eprintln!("Request_failure errored: {:?}", &err);
                }
            }
            err
        })
        .map_ok(move |mut res| {
            for mw in mws_success.lock().unwrap().iter_mut() {
                match mw.request_success(&mut res, &context, &state_success) {
                    Err(err) => res = Response::from(err),
                    Ok(RespondWith(response)) => res = response,
                    Ok(Next) => ()
                }
            }
            res
        })
        .map_ok_or_else(
            move |err| {
                let mut res = Err(err);
                for mw in mws_after_success.lock().unwrap().iter_mut() {
                    match mw.after_request(None, &context, &state_after_success) {
                        Err(err) => res = Ok(Response::from(err)),
                        Ok(RespondWith(response)) => res = Ok(response),
                        Ok(Next) => ()
                    }
                }
                res
            },
            move |mut res| {
                for mw in mws_after_failure.lock().unwrap().iter_mut() {
                    match mw.after_request(Some(&mut res), &context, &state_after_failure) {
                        Err(err) => res = Response::from(err),
                        Ok(RespondWith(response)) => res = response,
                        Ok(Next) => ()
                    }
                }
                Ok(res)
            }
        );
        Box::pin(res)
    }
}

impl ProxyService {
    fn early_response(
        &self, context: &ServiceContext,
        mut res: Response<Body>, state: &State
    ) -> Response<Body> {
        for mw in self.middlewares.lock().unwrap().iter_mut() {
            match mw.after_request(Some(&mut res), context, state) {
                Err(err) => res = Response::from(err),
                Ok(RespondWith(response)) => res = response,
                Ok(Next) => ()
            }
        }
        println!("Early response is {:?}", &res);
        res
    }

    fn clear_state(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.clear();
        } else {
            eprintln!("[FATAL] Cannot lock state in clean_stale_state");
        }
    }

    pub fn new(middlewares: Middlewares, remote_addr: SocketAddr) -> Self {
        ProxyService {
            client: Client::new(),
            state: Arc::new(Mutex::new(HashMap::new())),
            rng: SmallRng::from_entropy(),
            remote_addr,
            middlewares
        }
    }
}