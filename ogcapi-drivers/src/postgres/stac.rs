use ogcapi_types::{
    common::{Bbox, Datetime, IntervalDatetime},
    features::{Feature, FeatureCollection},
    stac::SearchParams,
};

use crate::StacSeach;

use super::Db;

#[async_trait::async_trait]
impl StacSeach for Db {
    async fn search(&self, query: &SearchParams) -> anyhow::Result<FeatureCollection> {
        let mut tx = self.pool.begin().await?;

        // WITH
        let mut collection_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM meta.collections 
            WHERE collection ->> 'type' = 'Collection'
            "#,
        )
        .fetch_all(&mut tx)
        .await?;

        if let Some(collections) = &query.collections {
            collection_ids = collections
                .iter()
                .filter(|c| collection_ids.contains(c))
                .map(|c| c.to_owned())
                .collect();
        }

        let union_all_items = collection_ids
            .iter()
            .map(|collection_id| {
                format!(
                    r#"
                    SELECT * FROM items."{collection_id}"
                    "#
                )
            })
            .collect::<Vec<String>>()
            .join(" UNION ALL ");

        // WHERE
        let mut where_conditions = vec!["TRUE".to_string()];

        // bbox
        if let Some(bbox) = query.bbox.as_ref() {
            let envelope = match bbox {
                Bbox::Bbox2D(bbox) => format!(
                    "ST_MakeEnvelope({}, {}, {}, {}, 4326)",
                    bbox[0], bbox[1], bbox[2], bbox[3]
                ),
                Bbox::Bbox3D(bbox) => format!(
                    "ST_MakeEnvelope({}, {}, {}, {}, 4326)",
                    bbox[0], bbox[1], bbox[3], bbox[4]
                ),
            };
            where_conditions.push(format!("geom && {}", envelope));
        }

        // datetime
        if let Some(datetime) = query.datetime.as_ref() {
            let (from, to) = match datetime {
                Datetime::Datetime(_) => (
                    format!("CAST('{datetime}' AS timestamptz)"),
                    format!("CAST('{datetime}' AS timestamptz)"),
                ),
                Datetime::Interval { from, to } => {
                    let from = match from {
                        IntervalDatetime::Datetime(_) => {
                            format!("CAST('{from}' AS timestamptz)")
                        }
                        IntervalDatetime::Open => "to_timestamp('-infinity')".to_owned(),
                    };
                    let to = match to {
                        IntervalDatetime::Datetime(_) => {
                            format!("CAST('{to}' AS timestamptz)")
                        }
                        IntervalDatetime::Open => "NOW()".to_owned(),
                    };
                    (from, to)
                }
            };

            where_conditions.push(format!(
                r#"
                (
                    CASE
                        WHEN (properties->'datetime') IS NOT NULL THEN (
                            CAST(properties->>'datetime' AS timestamptz)
                            BETWEEN {from} AND {to}
                        )
                        WHEN (
                            (properties->'datetime') IS NULL
                            AND (properties->'start_datetime') IS NOT NULL
                            AND (properties->'end_datetime') IS NOT NULL
                        ) THEN (
                            ({from}, {to}) OVERLAPS (
                                CAST(properties->>'start_datetime' AS timestamptz),
                                CAST(properties->>'end_datetime' AS timestamptz)
                            )
                        )
                        ELSE TRUE
                    END
                )
                "#
            ));
        }

        // ids
        if let Some(ids) = query.ids.as_ref() {
            where_conditions.push(format!("id IN ('{}')", ids.join("','")))
        }

        // intersects
        if let Some(intersects) = query.intersects.as_ref() {
            where_conditions.push(format!("geom && ST_GeomFromGeoJSON('{}')", intersects));
        }

        let conditions = where_conditions.join(" AND ");

        // COUNT
        let number_matched: (i64,) = sqlx::query_as(&format!(
            r#"
            WITH items AS ({union_all_items})
            SELECT count(*) FROM items
            WHERE {conditions}
            "#,
        ))
        .fetch_one(&mut tx)
        .await?;

        // FETCH
        let features: Option<sqlx::types::Json<Vec<Feature>>> = sqlx::query_scalar(&format!(
            r#"
            WITH items AS ({union_all_items})
            SELECT array_to_json(array_agg(row_to_json(t)))
            FROM (
                SELECT
                    id,
                    collection,
                    properties,
                    ST_AsGeoJSON(ST_Transform(geom, 4326))::jsonb as geometry,
                    links,
                    assets,
                    bbox
                FROM items
                WHERE {conditions}
                LIMIT {}
                OFFSET {}
            ) t
            "#,
            query
                .limit
                .map_or_else(|| String::from("NULL"), |l| l.to_string()),
            query.offset.unwrap_or(0)
        ))
        .fetch_one(&mut tx)
        .await?;

        tx.commit().await?;

        let features = features.map(|f| f.0).unwrap_or_default();
        let mut fc = FeatureCollection::new(features);
        fc.number_matched = Some(number_matched.0 as u64);

        Ok(fc)
    }
}
