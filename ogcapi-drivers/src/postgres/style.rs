use ogcapi_types::styles::{Style, Styles, Stylesheet};

use crate::StyleTransactions;

use super::Db;

#[async_trait::async_trait]
impl StyleTransactions for Db {
    async fn list_styles(&self) -> anyhow::Result<Styles> {
        let styles: Option<sqlx::types::Json<Vec<Style>>> = sqlx::query_scalar(
            r#"
            SELECT array_to_json(array_agg(row_to_json(t)))
            FROM (
                SELECT id, title, links FROM meta.styles
            ) t
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let styles = styles.map(|s| s.0).unwrap_or_default();
        Ok(Styles { styles })
    }

    async fn read_style(&self, id: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let style: Option<sqlx::types::Json<Stylesheet>> = sqlx::query_scalar(
            r#"
            SELECT row_to_json(t) as "stylesheet!"
            FROM (
                SELECT id, value FROM meta.styles WHERE id = $1
            ) t
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(style.map(|s| s.0.value))
    }
}
