INSERT INTO collections (
    id,
    title,
    description,
    links,
    extent,
    collection_type,
    crs,
    storage_crs,
    storage_crs_coordinate_epoch,
    stac_version,
    stac_extensions,
    keywords,
    licence,
    providers,
    summaries
    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
RETURNING
    id,
    title,
    description,
    links AS "links: Json<Vec<Link>>",
    extent AS "extent: Json<Extent>",
    collection_type AS "collection_type: CollectionType",
    crs,
    storage_crs,
    storage_crs_coordinate_epoch,
    stac_version,
    stac_extensions,
    keywords,
    licence,
    providers AS "providers: Json<Vec<Provider>>",
    summaries AS "summaries: Json<Summaries>"
