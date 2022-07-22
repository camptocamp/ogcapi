mod args;
mod boundaries;
pub mod geojson;
pub mod ogr;
pub mod osm;

pub use args::Args;

#[cfg(feature = "stac")]
pub(crate) async fn load_asset_from_path(
    path: &std::path::PathBuf,
) -> anyhow::Result<std::collections::HashMap<String, ogcapi_types::stac::Asset>> {
    use ogcapi_drivers::s3::{ByteStream, S3};
    use ogcapi_types::common::media_type::GEO_JSON;

    // Setup S3 driver
    let s3 = S3::new().await;

    let stream = ByteStream::from_path(&path).await?;

    // Upload asset
    let filename = path.file_name().unwrap().to_str().unwrap();

    let key = format!("assets/{filename}");

    s3.client
        .put_object()
        .bucket(std::env::var("AWS_S3_BUCKET_NAME")?)
        .key(&key)
        .body(stream)
        .content_type(GEO_JSON)
        .send()
        .await?;

    let asset = ogcapi_types::stac::Asset::new(key);

    let file_stem = path.file_stem().unwrap().to_str().unwrap();

    Ok(std::collections::HashMap::from([(
        file_stem.to_string(),
        asset,
    )]))
}

pub(crate) async fn bulk_load_items(
    collection: &str,
    ids: &[String],
    properties: &[Option<sqlx::types::Json<serde_json::Map<String, serde_json::Value>>>],
    geoms: &[Vec<u8>],
    connection: &sqlx::PgPool,
) -> Result<(), sqlx::Error> {
    let batch_size = 10000;
    let total = geoms.len();

    let mut start = 0;
    let mut end = start + batch_size;

    let mut ids_batch;
    let mut properties_batch;
    let mut geoms_batch;

    while start < total {
        if end < total {
            ids_batch = &ids[start..end];
            properties_batch = &properties[start..end];
            geoms_batch = &geoms[start..end];
        } else {
            ids_batch = &ids[start..];
            properties_batch = &properties[start..];
            geoms_batch = &geoms[start..];
        }
        sqlx::query(&format!(
            r#"
            INSERT INTO items."{}" (id, properties, geom)
            SELECT * FROM UNNEST($1::text[], $2::jsonb[], $3::bytea[])
            "#,
            collection
        ))
        .bind(ids_batch)
        .bind(properties_batch)
        .bind(geoms_batch)
        .execute(connection)
        .await?;

        start = end;
        end += batch_size;
    }

    Ok(())
}
