use std::sync::Arc;

use anyhow::Context;
use axum::{
    extract::{Extension, Path},
    http::{
        header::{CONTENT_TYPE, LOCATION},
        HeaderMap, StatusCode,
    },
    routing::get,
    Json, Router,
};

use ogcapi_types::{
    common::{
        link_rel::{COLLECTION, NEXT, PREV, ROOT, SELF},
        media_type::{GEO_JSON, JSON},
        Collection, Crs, Link, Linked,
    },
    features::{Feature, FeatureCollection, Query},
};

use crate::{
    extractors::{Qs, RemoteUrl},
    Error, Result, State,
};

const CONFORMANCE: [&str; 4] = [
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
    "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson",
    "http://www.opengis.net/spec/ogcapi-features-2/1.0/conf/crs",
];

async fn create(
    Path(collection_id): Path<String>,
    Json(mut feature): Json<Feature>,
    RemoteUrl(url): RemoteUrl,
    Extension(state): Extension<Arc<State>>,
) -> Result<(StatusCode, HeaderMap)> {
    feature.collection = Some(collection_id);

    let id = state.drivers.features.create_feature(&feature).await?;

    let location = url.join(&format!("items/{}", id))?;

    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, location.as_str().parse().unwrap());

    Ok((StatusCode::CREATED, headers))
}

async fn read(
    Path((collection_id, id)): Path<(String, String)>,
    Qs(query): Qs<Query>,
    RemoteUrl(url): RemoteUrl,
    Extension(state): Extension<Arc<State>>,
) -> Result<(HeaderMap, Json<Feature>)> {
    let collection = state
        .drivers
        .collections
        .read_collection(&collection_id)
        .await?
        .ok_or(Error::NotFound)?;
    is_supported_crs(&collection, &query.crs).await?;

    let mut feature = state
        .drivers
        .features
        .read_feature(&collection_id, &id, &query.crs)
        .await?
        .ok_or(Error::NotFound)?;

    feature.links.insert_or_update(&[
        Link::new(&url, SELF).mediatype(GEO_JSON),
        Link::new(&url.join("../../..")?, ROOT).mediatype(JSON),
        Link::new(&url.join(&format!("../../{}", collection_id))?, COLLECTION).mediatype(JSON),
    ]);
    feature.links.resolve_relative_links();

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Crs",
        query
            .crs
            .to_string()
            .parse()
            .context("Unable to parse `Content-Crs` header value")?,
    );
    headers.insert(CONTENT_TYPE, GEO_JSON.parse().unwrap());

    Ok((headers, Json(feature)))
}

async fn update(
    Path((collection_id, id)): Path<(String, String)>,
    Json(mut feature): Json<Feature>,
    Extension(state): Extension<Arc<State>>,
) -> Result<StatusCode> {
    feature.id = Some(id);
    feature.collection = Some(collection_id);

    state.drivers.features.update_feature(&feature).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    Path((collection_id, id)): Path<(String, String)>,
    Extension(state): Extension<Arc<State>>,
) -> Result<StatusCode> {
    state
        .drivers
        .features
        .delete_feature(&collection_id, &id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn items(
    Path(collection_id): Path<String>,
    Qs(mut query): Qs<Query>,
    RemoteUrl(mut url): RemoteUrl,
    Extension(state): Extension<Arc<State>>,
) -> Result<(HeaderMap, Json<FeatureCollection>)> {
    tracing::debug!("{:#?}", query);

    // Limit
    if let Some(limit) = query.limit {
        if limit > 10000 {
            query.limit = Some(10000);
        }
    } else {
        query.limit = Some(100);
    }

    let collection = state
        .drivers
        .collections
        .read_collection(&collection_id)
        .await?
        .ok_or(Error::NotFound)?;
    is_supported_crs(&collection, &query.crs).await?;

    // TODO: validate additional parameters

    let mut fc = state
        .drivers
        .features
        .list_items(&collection_id, &query)
        .await?;

    fc.links.insert_or_update(&[
        Link::new(&url, SELF).mediatype(GEO_JSON),
        Link::new(&url.join("../..")?, ROOT).mediatype(JSON),
        Link::new(&url.join(".")?, COLLECTION).mediatype(JSON),
    ]);

    // pagination
    if let Some(limit) = query.limit {
        if query.offset.is_none() {
            query.offset = Some(0);
        }

        if let Some(offset) = query.offset {
            if offset != 0 && offset >= limit {
                query.offset = Some(offset - limit);
                url.set_query(serde_qs::to_string(&query).ok().as_deref());
                let previous = Link::new(&url, PREV).mediatype(GEO_JSON);
                fc.links.insert_or_update(&[previous]);
            }

            if let Some(number_matched) = fc.number_matched {
                if number_matched > (offset + limit) as u64 {
                    query.offset = Some(offset + limit);
                    url.set_query(serde_qs::to_string(&query).ok().as_deref());
                    let next = Link::new(&url, NEXT).mediatype(GEO_JSON);
                    fc.links.insert_or_update(&[next]);
                }
            }
        }
    }

    for feature in fc.features.iter_mut() {
        feature.links.insert_or_update(&[
            Link::new(
                &url.join(&format!("items/{}", feature.id.as_ref().unwrap()))?,
                SELF,
            )
            .mediatype(GEO_JSON),
            Link::new(&url.join("../..")?, ROOT).mediatype(JSON),
            Link::new(&url.join(&format!("../{}", collection.id))?, COLLECTION).mediatype(JSON),
        ])
    }

    let mut headers = HeaderMap::new();
    headers.insert("Content-Crs", query.crs.to_string().parse().unwrap());
    headers.insert(CONTENT_TYPE, GEO_JSON.parse().unwrap());

    Ok((headers, Json(fc)))
}

async fn is_supported_crs(collection: &Collection, crs: &Crs) -> Result<(), Error> {
    if collection.crs.contains(crs) {
        Ok(())
    } else {
        Err(Error::Exception(
            StatusCode::BAD_REQUEST,
            format!("Unsuported CRS `{}`", crs),
        ))
    }
}

pub(crate) fn router(state: &State) -> Router {
    state.conformance.write().unwrap().extend(&CONFORMANCE);

    Router::new()
        .route("/collections/:collection_id/items", get(items).post(create))
        .route(
            "/collections/:collection_id/items/:id",
            get(read).put(update).delete(remove),
        )
}
