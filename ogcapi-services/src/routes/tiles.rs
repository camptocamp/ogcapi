use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Deserialize;

use ogcapi_types::{
    common::{
        link_rel::{TILESETS_VECTOR, TILING_SCHEME},
        media_type::JSON,
        Link,
    },
    tiles::{Query, TileMatrix, TileMatrixSet, TileMatrixSetItem, TileMatrixSets, TileSets},
};

use crate::{
    extractors::{Qs, RemoteUrl},
    Error, Result, State,
};

const CONFORMANCE: [&str; 7] = [
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/core",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/tileset",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/tilesets-list",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/geodata-tilesets",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/dataset-tilesets",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/geodata-selection",
    // "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/jpeg",
    // "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/png",
    "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/mvt",
    // "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/geojson",
    // "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/tiff",
    // "http://www.opengis.net/spec/ogcapi-tiles-1/1.0/conf/netcdf",
];

const WEB_MERCARTOR_QUAD: &[u8; 8005] = include_bytes!("../../assets/tms/WebMercartorQuad.json");

static TMS: OnceCell<HashMap<String, TileMatrixSet>> = OnceCell::new();
static TM: OnceCell<HashMap<String, HashMap<String, TileMatrix>>> = OnceCell::new();

#[derive(Deserialize, Debug)]
pub struct TileParams {
    collection_id: Option<String>,
    /// Identifier selecting one of the TileMatrixSetId supported by the resource.
    tms_id: String,
    /// Identifier selecting one of the scales defined in the TileMatrixSet
    /// and representing the scaleDenominator the tile.
    matrix: String,
    /// Row index of the tile on the selected TileMatrix. It cannot exceed
    /// the MatrixWidth-1 for the selected TileMatrix.
    row: u32,
    /// Column index of the tile on the selected TileMatrix. It cannot exceed
    /// the MatrixHeight-1 for the selected TileMatrix.
    col: u32,
}

async fn tile_matrix_sets(RemoteUrl(url): RemoteUrl) -> Result<Json<TileMatrixSets>> {
    let tms = TMS.get().expect("TMS cell to be inizialized");

    let mut tile_matrix_sets = Vec::new();

    for tms in tms.values() {
        let item = TileMatrixSetItem {
            id: Some(tms.id.to_owned()),
            title: tms.title_description_keywords.title.to_owned(),
            links: vec![Link::new(
                &url.join(&format!("tileMatrixSets/{}", &tms.id))?,
                TILING_SCHEME,
            )],
            ..Default::default()
        };

        tile_matrix_sets.push(item);
    }

    Ok(Json(TileMatrixSets { tile_matrix_sets }))
}

async fn tile_matrix_set(Path(id): Path<String>) -> Result<Json<TileMatrixSet>> {
    match TMS.get().and_then(|tms| tms.get(&id)) {
        Some(tms) => Ok(Json(tms.to_owned())),
        None => Err(Error::Exception(
            StatusCode::NOT_FOUND,
            "Unable to find resource".to_string(),
        )),
    }
}

async fn tiles() -> Result<Json<TileSets>> {
    let tile_sets = TileSets {
        tilesets: vec![],
        links: None,
    };

    Ok(Json(tile_sets))
}

async fn tile(
    Path(params): Path<TileParams>,
    Qs(query): Qs<Query>,
    Extension(state): Extension<Arc<State>>,
) -> Result<Vec<u8>> {
    let tms = TMS
        .get()
        .and_then(|tms| tms.get(&params.tms_id))
        .expect("Get tms from TMS");

    let tiles = state
        .drivers
        .tiles
        .tile(
            &params.collection_id.or(query.collections).unwrap(),
            tms,
            &params.matrix,
            params.row,
            params.col,
        )
        .await?;

    Ok(tiles)
}

pub(crate) fn router(state: &State) -> Router {
    let mut root = state.root.write().unwrap();
    root.links.push(
        Link::new("tiles", TILESETS_VECTOR)
            .title("List of available vector features tilesets for the dataset")
            .mediatype(JSON),
    );

    state.conformance.write().unwrap().extend(&CONFORMANCE);

    // Setup tile matrix sets
    let mut tms_map = HashMap::new();
    let web_mercartor_quad: TileMatrixSet =
        serde_json::from_slice(WEB_MERCARTOR_QUAD).expect("parse tms");
    tms_map.insert(web_mercartor_quad.id.to_owned(), web_mercartor_quad);

    let mut tm = HashMap::new();
    for tms in tms_map.values() {
        tm.insert(tms.id.to_owned(), HashMap::new());
        for tile_matrix in &tms.tile_matrices {
            tm.get_mut(&tms.id).and_then(|tm_map| {
                tm_map.insert(tile_matrix.id.to_owned(), tile_matrix.to_owned())
            });
        }
    }
    TMS.set(tms_map).expect("set `TMS` once cell content");
    TM.set(tm).expect("set `TM` once cell content");

    Router::new()
        .route("/tileMatrixSets", get(tile_matrix_sets))
        .route("/tileMatrixSets/:tms_id", get(tile_matrix_set))
        .route("/tiles", get(tiles))
        .route("/tiles/:tms_id", get(tile_matrix_set))
        .route("/tiles/:tms_id/:matrix/:row/:col", get(tile))
        // .route("/collections/:collection_id/tiles", get(tiles))
        .route(
            "/collections/:collection_id/tiles/:tms_id/:matrix/:row/:col",
            get(tile),
        )
}
