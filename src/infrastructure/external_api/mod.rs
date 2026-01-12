//! External API infrastructure module

mod service;

pub use service::{
    CreateExternalApiRequest, ExternalApiService, ExternalApiServiceTrait, UpdateExternalApiRequest,
};
