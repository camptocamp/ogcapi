pub mod routes;

pub use routes::{collections, features, tiles};

mod exception;

use sqlx::{postgres::PgPoolOptions, PgPool};
use tide::{self, utils::After};

#[derive(Clone)]
pub struct Service {
    pub pool: PgPool,
}

impl Service {
    pub async fn new() -> Self {
        Service {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect(&std::env::var("DATABASE_URL").expect("Read database url"))
                .await
                .expect("Create db connection pool"),
        }
    }

    pub async fn run(self, url: &str) -> tide::Result<()> {
        tide::log::start();
        let mut app = tide::with_state(self);

        // core
        app.at("/").get(routes::root);
        app.at("/api").get(routes::api);
        app.at("/conformance").get(routes::conformance);

        // favicon
        app.at("/favicon.ico").serve_file("favicon.ico")?;

        // redoc
        app.at("/redoc").get(routes::redoc);

        // queryables
        //app.at("/queryables").get(handle_queryables);

        // Collections
        app.at("/collections")
            .get(collections::handle_collections)
            .post(collections::create_collection);
        app.at("/collections/:collection")
            .get(collections::read_collection)
            .put(collections::update_collection)
            .delete(collections::delete_collection);
        //app.at("/collections/:collection/queryables").get(handle_queryables);

        // Features
        app.at("/collections/:collection/items")
            .get(features::handle_items)
            .post(features::create_item);
        app.at("/collections/:collection/items/:id")
            .get(features::read_item)
            .put(features::update_item)
            .delete(features::delete_item);

        // Tiles
        // app.at("tileMatrixSets").get(tiles::get_matrixsets);
        // app.at("tileMatrixSets/:matrix_set").get(tiles::get_matrixset);
        // app.at("collections/:collection/tiles").get(tiles::handle_tiles);
        app.at("collections/:collection/tiles/:matrix_set/:matrix/:row/:col")
            .get(tiles::get_tile);

        app.with(After(exception::exception));

        app.listen(url).await?;
        Ok(())
    }
}