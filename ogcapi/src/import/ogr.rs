use std::{collections::HashSet, path::PathBuf};

use gdal::{
    spatial_ref::{CoordTransform, SpatialRef},
    vector::FieldValue,
};
use serde_json::{Map, Value};

use ogcapi_drivers::{postgres::Db, CollectionTransactions};
use ogcapi_types::common::{Bbox, Collection, Crs, Extent, SpatialExtent};

use super::Args;

pub async fn load(mut args: Args) -> Result<(), anyhow::Error> {
    // Setup a db connection pool
    let db = Db::setup(&args.database_url).await?;

    // Handle http & zip
    if args.input.starts_with("http") && args.input.extension() == Some(std::ffi::OsStr::new("zip"))
    {
        args.input = PathBuf::from(format!(
            "/vsizip//vsicurl/{}",
            args.input.as_path().to_str().unwrap()
        ))
    } else if args.input.display().to_string().starts_with("http") {
        args.input = PathBuf::from("/vsicurl/").join(args.input.as_path())
    } else if args.input.extension() == Some(std::ffi::OsStr::new("zip")) {
        args.input = PathBuf::from("/vsizip/").join(args.input.as_path())
    }

    let dataset = gdal::Dataset::open(&args.input)?;

    if dataset.layer_count() > 1 && args.filter.is_none() {
        tracing::warn!(
            "Found multiple layers! Use the '--filter' option to specifiy one of:\n\t- {}",
            dataset
                .layers()
                .map(|l| l.name())
                .collect::<Vec<String>>()
                .join("\n\t- ")
        );
        return Ok(());
    }

    for mut layer in dataset.layers() {
        // Skip additional layers
        if args.filter.is_some() && Some(layer.name()) != args.filter {
            tracing::debug!("Skipping layer `{}`", layer.name());
            continue;
        }

        // Prepare the origin and destination spatial references objects and coordinate transformation
        let spatial_ref_src = match args.s_srs {
            Some(epsg) => SpatialRef::from_epsg(epsg)?,
            None => layer.spatial_ref()?,
        };

        let spatial_ref_dst = match args.t_srs {
            Some(epsg) => SpatialRef::from_epsg(epsg)?,
            None => layer.spatial_ref()?,
        };

        spatial_ref_src.set_axis_mapping_strategy(0);
        spatial_ref_dst.set_axis_mapping_strategy(0);

        let transform = CoordTransform::new(&spatial_ref_src, &spatial_ref_dst)?;

        // Create collection
        let storage_crs = Crs::from_srid(spatial_ref_dst.auth_code()?);

        let collection = Collection {
            id: args.collection.to_owned(),
            crs: Vec::from_iter(HashSet::from([
                Crs::default(),
                storage_crs.clone(),
                Crs::from_epsg(3857),
                Crs::from_epsg(2056),
            ])),
            extent: layer
                .try_get_extent()?
                .map(|e| {
                    let mut x = [e.MinX, e.MaxX];
                    let mut y = [e.MinY, e.MaxY];
                    // let mut z = [e.MinZ, e.MaxZ];
                    transform
                        .transform_coords(&mut x, &mut y, &mut [])
                        .expect("Transform extent coords");
                    Extent {
                        spatial: Some(SpatialExtent {
                            bbox: vec![Bbox::Bbox2D([x[0], y[0], x[1], y[1]])],
                            crs: spatial_ref_dst
                                .auth_code()
                                .map(Crs::from_srid)
                                .unwrap_or_default(),
                        }),
                        temporal: None,
                    }
                })
                .or_else(|| Some(Extent::default())),
            storage_crs: Some(storage_crs.to_owned()),
            #[cfg(feature = "stac")]
            assets: crate::import::load_asset_from_path(&args.input).await?,
            ..Default::default()
        };

        db.delete_collection(&collection.id).await?;
        db.create_collection(&collection).await?;

        // Set concrete geometry type if possible https://github.com/georust/gdal/blob/00adecc94361228a2197224205fc9260d14d7549/gdal-sys/prebuilt-bindings/gdal_3.4.rs#L3454
        if let Some((geometry_type, dimensions)) =
            match layer.defn().geom_fields().next().unwrap().field_type() {
                0 => {
                    tracing::debug!("Unknown gemetry type.");
                    None
                }
                1 => Some(("POINT", 2)),
                2 => Some(("LINESTRING", 2)),
                3 => Some(("POLYGON", 2)),
                4 => Some(("MULTIPOINT", 2)),
                5 => Some(("MULTILINESTRING", 2)),
                6 => Some(("MULTIPOLYGON", 2)),
                2147483653 => Some(("MULTILINESTRINGZ", 3)),
                2147483654 => Some(("MULTIPOLYGONZ", 3)),
                i => {
                    tracing::warn!("Unmaped geometry type `{i}`");
                    None
                }
            }
        {
            sqlx::query("SELECT DropGeometryColumn ('items', $1, 'geom')")
                .bind(&collection.id)
                .execute(&db.pool)
                .await?;
            sqlx::query("SELECT AddGeometryColumn ('items', $1, 'geom', $2, $3, $4)")
                .bind(&collection.id)
                .bind(&storage_crs.as_srid())
                .bind(geometry_type)
                .bind(dimensions)
                .execute(&db.pool)
                .await?;
            sqlx::query(&format!(
                r#"CREATE INDEX ON items."{}" USING gist (geom)"#,
                collection.id
            ))
            .execute(&db.pool)
            .await?;
        }

        // Load features
        let now = std::time::Instant::now();
        let count = layer.feature_count();

        let field_names: Vec<String> = layer.defn().fields().map(|f| f.name()).collect();

        let mut ids = Vec::new();
        let mut properties = Vec::new();
        let mut geoms = Vec::new();

        for (i, feature) in layer.features().enumerate() {
            // identifier
            let id = feature.fid().unwrap_or(i as u64).to_string();
            ids.push(id);

            // properties
            let mut attributes = Map::new();

            for field_name in &field_names {
                let value = if let Some(value) = feature.field(field_name)? {
                    match value {
                        FieldValue::IntegerValue(v) => Value::from(v),
                        FieldValue::IntegerListValue(v) => Value::from(v),
                        FieldValue::Integer64Value(v) => Value::from(v),
                        FieldValue::Integer64ListValue(v) => Value::from(v),
                        FieldValue::StringValue(v) => Value::from(v),
                        FieldValue::StringListValue(v) => Value::from(v),
                        FieldValue::RealValue(v) => Value::from(v),
                        FieldValue::RealListValue(v) => Value::from(v),
                        FieldValue::DateValue(v) => Value::from(v.naive_utc().to_string()),
                        FieldValue::DateTimeValue(v) => Value::from(v.to_rfc3339()),
                    }
                } else {
                    Value::Null
                };
                attributes.insert(field_name.to_owned(), value);
            }
            properties.push(Some(sqlx::types::Json(attributes)));

            // geometry
            let geom = feature.geometry();
            let geom = geom.transform(&transform)?;
            geoms.push(geom.wkb()?);
        }

        // load
        super::bulk_load_items(&collection.id, &ids, &properties[..], &geoms[..], &db.pool).await?;

        // stats
        let elapsed = now.elapsed().as_millis() as f64 / 1000.0;
        tracing::info!(
            "Loaded {count} features in {elapsed} seconds ({:.2}/s)",
            count as f64 / elapsed
        );
    }

    Ok(())
}
