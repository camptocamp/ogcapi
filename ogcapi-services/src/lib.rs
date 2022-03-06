mod error;
mod extractors;
mod routes;

pub use error::Error;

use std::sync::{Arc, RwLock};

use axum::extract::Extension;
use axum::{routing::get, Router};
use openapiv3::OpenAPI;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use ogcapi_drivers::postgres::Db;
use ogcapi_entities::common::{Conformance, LandingPage, Link, LinkRel, MediaType};

pub type Result<T, E = Error> = std::result::Result<T, E>;

static OPENAPI: &[u8; 29680] = include_bytes!("../openapi.yaml");

#[derive(Clone)]
pub struct State {
    db: Db,
    // collections: Arc<RwLock<HashMap<String, Collection>>>,
    root: Arc<RwLock<LandingPage>>,
    conformance: Arc<RwLock<Conformance>>,
    openapi: Arc<OpenAPI>,
}

pub async fn server(db: Db) -> Router {
    // state
    let openapi: OpenAPI = serde_yaml::from_slice(OPENAPI).unwrap();

    let root = Arc::new(RwLock::new(LandingPage {
        title: Some(openapi.info.title.clone()),
        description: openapi.info.description.clone(),
        links: vec![
            Link::new("http://ogcapi.rs/")
                .title("This document".to_string())
                .mime(MediaType::JSON),
            Link::new("http://ogcapi.rs/api")
                .title("The Open API definition".to_string())
                .relation(LinkRel::ServiceDesc)
                .mime(MediaType::OpenAPIJson),
            Link::new("http://ogcapi.rs/conformance")
                .title("OGC conformance classes implemented by this API".to_string())
                .relation(LinkRel::Conformance)
                .mime(MediaType::JSON),
            Link::new("http://ogcapi.rs/collections")
                .title("Metadata about the resource collections".to_string())
                .relation(LinkRel::Data)
                .mime(MediaType::JSON),
        ],
        ..Default::default()
    }));

    let conformance = Arc::new(RwLock::new(Conformance {
        conforms_to: vec![
            "http://www.opengis.net/spec/ogcapi-common-1/1.0/req/core".to_string(),
            "http://www.opengis.net/spec/ogcapi-common-2/1.0/req/collections".to_string(),
            "http://www.opengis.net/spec/ogcapi_common-2/1.0/req/json".to_string(),
        ],
    }));

    let state = State {
        db,
        root,
        conformance,
        openapi: Arc::new(openapi),
    };

    // routes
    let router = Router::new()
        .route("/", get(routes::root))
        .route("/api", get(routes::api))
        .route("/redoc", get(routes::redoc))
        .route("/conformance", get(routes::conformance))
        .route(
            "/favicon.ico",
            get(|| async move { include_bytes!("../favicon.ico").to_vec() }),
        )
        .merge(routes::collections::router(&state))
        .merge(routes::features::router(&state))
        .merge(routes::tiles::router(&state))
        .merge(routes::styles::router(&state));

    #[cfg(feature = "processes")]
    let router = router.merge(routes::processes::router(&state));

    #[cfg(feature = "edr")]
    let router = router.merge(routes::edr::router(&state));

    router.layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
            .layer(Extension(state)),
    )
}