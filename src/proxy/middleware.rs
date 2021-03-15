use hyper::{Body, Error, Request, Response};

pub enum MiddlewareResult {
    RespondWith(Response<hyper::Body>),
    Next
}