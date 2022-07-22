use ogcapi_types::common::{Collection, Collections, Query};

use crate::CollectionTransactions;

use super::Db;

#[async_trait::async_trait]
impl CollectionTransactions for Db {
    async fn create_collection(&self, collection: &Collection) -> anyhow::Result<String> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(&format!(
            r#"
            CREATE TABLE items."{0}" (
                id text PRIMARY KEY DEFAULT gen_random_uuid()::text,
                collection text REFERENCES meta.collections(id) DEFAULT '{0}',
                properties jsonb,
                geom geometry NOT NULL,
                links jsonb NOT NULL DEFAULT '[]'::jsonb,
                assets jsonb NOT NULL DEFAULT '{{}}'::jsonb,
                bbox jsonb
            )
            "#,
            collection.id
        ))
        .execute(&mut tx)
        .await?;

        sqlx::query(&format!(
            r#"CREATE INDEX ON items."{}" USING btree (collection)"#,
            collection.id
        ))
        .execute(&mut tx)
        .await?;

        sqlx::query(&format!(
            r#"CREATE INDEX ON items."{}" USING gin (properties)"#,
            collection.id
        ))
        .execute(&mut tx)
        .await?;

        sqlx::query(&format!(
            r#"CREATE INDEX ON items."{}" USING gist (geom)"#,
            collection.id
        ))
        .execute(&mut tx)
        .await?;

        sqlx::query("SELECT UpdateGeometrySRID('items', $1, 'geom', $2)")
            .bind(&collection.id)
            .bind(collection.storage_crs.clone().unwrap_or_default().as_srid())
            .execute(&mut tx)
            .await?;

        sqlx::query("INSERT INTO meta.collections ( id, collection ) VALUES ( $1, $2 )")
            .bind(&collection.id)
            .bind(sqlx::types::Json(collection))
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(collection.id.to_owned())
    }

    async fn read_collection(&self, id: &str) -> anyhow::Result<Option<Collection>> {
        // TODO: cache
        let collection: Option<sqlx::types::Json<Collection>> = sqlx::query_scalar(
            r#"
            SELECT collection as "collection!" 
            FROM meta.collections WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(collection.map(|c| c.0))
    }

    async fn update_collection(&self, collection: &Collection) -> anyhow::Result<()> {
        sqlx::query("UPDATE meta.collections SET collection = $2 WHERE id = $1")
            .bind(&collection.id)
            .bind(sqlx::types::Json(collection))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_collection(&self, id: &str) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(&format!(r#"DROP TABLE IF EXISTS items."{}""#, id))
            .execute(&mut tx)
            .await?;

        sqlx::query("DELETE FROM meta.collections WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn list_collections(&self, _query: &Query) -> anyhow::Result<Collections> {
        let collections: Option<sqlx::types::Json<Vec<Collection>>> = sqlx::query_scalar(
            r#"
            SELECT array_to_json(array_agg(collection))
            FROM meta.collections
            WHERE collection ->> 'type' = 'Collection'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let collections = collections.map(|c| c.0).unwrap_or_default();
        let mut collections = Collections::new(collections);
        collections.number_matched = collections.number_returned;

        Ok(collections)
    }
}
