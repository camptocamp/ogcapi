#[cfg(feature = "processes")]
use std::collections::BTreeMap;
use std::sync::RwLock;

#[cfg(feature = "edr")]
use ogcapi_drivers::EdrQuerier;
#[cfg(feature = "features")]
use ogcapi_drivers::FeatureTransactions;
#[cfg(feature = "processes")]
use ogcapi_drivers::JobHandler;
#[cfg(feature = "styles")]
use ogcapi_drivers::StyleTransactions;
#[cfg(feature = "tiles")]
use ogcapi_drivers::TileTransactions;

use ogcapi_drivers::{postgres::Db, CollectionTransactions};
use ogcapi_types::common::{Conformance, LandingPage};

#[cfg(feature = "processes")]
use crate::Processor;
use crate::{openapi::OPENAPI, Config, ConfigParser, OpenAPI};

/// Application state
pub struct State {
    pub root: RwLock<LandingPage>,
    pub conformance: RwLock<Conformance>,
    pub openapi: OpenAPI,
    pub drivers: Drivers,
    pub db: Db,
    #[cfg(feature = "stac")]
    pub s3: ogcapi_drivers::s3::S3,
    #[cfg(feature = "processes")]
    pub processors: BTreeMap<String, Box<dyn Processor>>,
}

// TODO: Introduce service trait
pub struct Drivers {
    pub collections: Box<dyn CollectionTransactions>,
    #[cfg(feature = "features")]
    pub features: Box<dyn FeatureTransactions>,
    #[cfg(feature = "edr")]
    pub edr: Box<dyn EdrQuerier>,
    #[cfg(feature = "processes")]
    pub jobs: Box<dyn JobHandler>,
    #[cfg(feature = "styles")]
    pub styles: Box<dyn StyleTransactions>,
    #[cfg(feature = "tiles")]
    pub tiles: Box<dyn TileTransactions>,
}

impl State {
    pub async fn new() -> Self {
        let config = Config::parse();
        State::new_from(&config).await
    }

    pub async fn new_from(config: &Config) -> Self {
        let openapi = if let Some(path) = &config.openapi {
            OpenAPI::from_path(path).unwrap()
        } else {
            OpenAPI::from_slice(OPENAPI)
        };

        let db = Db::setup(&config.database_url).await.unwrap();

        State::new_with(db, openapi).await
    }

    pub async fn new_with(db: Db, openapi: OpenAPI) -> Self {
        // conformance
        #[allow(unused_mut)]
        let mut conformace = Conformance::default();
        #[cfg(feature = "stac")]
        conformace.extend(&[
            "https://api.stacspec.org/v1.0.0-rc.1/core",
            "https://api.stacspec.org/v1.0.0-rc.1/item-search",
            "https://api.stacspec.org/v1.0.0-rc.1/collections",
            "https://api.stacspec.org/v1.0.0-rc.1/ogcapi-features",
            "https://api.stacspec.org/v1.0.0-rc.1/browseable",
        ]);

        // drivers
        let drivers = Drivers {
            collections: Box::new(db.clone()),
            #[cfg(feature = "features")]
            features: Box::new(db.clone()),
            #[cfg(feature = "edr")]
            edr: Box::new(db.clone()),
            #[cfg(feature = "processes")]
            jobs: Box::new(db.clone()),
            #[cfg(feature = "styles")]
            styles: Box::new(db.clone()),
            #[cfg(feature = "tiles")]
            tiles: Box::new(db.clone()),
        };

        State {
            root: RwLock::new(LandingPage::new("root").description("root")),
            conformance: RwLock::new(conformace),
            openapi,
            drivers,
            db,
            #[cfg(feature = "stac")]
            s3: ogcapi_drivers::s3::S3::new().await,
            #[cfg(feature = "processes")]
            processors: Default::default(),
        }
    }

    pub fn root(mut self, root: LandingPage) -> Self {
        self.root = RwLock::new(root);
        self
    }

    pub fn openapi(mut self, openapi: OpenAPI) -> Self {
        self.openapi = openapi;
        self
    }

    #[cfg(feature = "stac")]
    pub async fn s3_client(mut self, client: ogcapi_drivers::s3::S3) -> Self {
        self.s3 = client;
        self
    }

    #[cfg(feature = "processes")]
    pub fn processors(mut self, processors: Vec<Box<dyn Processor>>) -> Self {
        for p in processors {
            self.processors.insert(p.id(), p);
        }
        self
    }
}
