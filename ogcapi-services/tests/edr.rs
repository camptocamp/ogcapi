#[cfg(feature = "edr")]
#[tokio::test]
async fn edr() -> anyhow::Result<()> {
    use std::env;
    use std::net::{SocketAddr, TcpListener};
    use std::path::PathBuf;
    use std::str::FromStr;

    use axum::http::Request;
    use geojson::{Geometry, Value};
    use ogcapi_drivers::postgres::Db;
    use sqlx::types::Json;
    use url::Url;
    use uuid::Uuid;

    use ogcapi_cli::import::{self, Args};
    use ogcapi_entities::common::Crs;
    use ogcapi_entities::edr::EdrQuery;
    use ogcapi_entities::features::FeatureCollection;

    // setup app
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let mut database_url = Url::parse(&env::var("DATABASE_URL")?)?;
    let daatbase_name = Uuid::new_v4().to_string();
    database_url.set_path(&daatbase_name);

    let db = Db::setup_with(&database_url, &daatbase_name, true)
        .await
        .expect("Setup database");

    let app = ogcapi_services::server(db).await;

    let listener = TcpListener::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .expect("")
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    let client = hyper::Client::new();

    // load data
    import::ogr::load(
        Args {
            input: PathBuf::from_str("../ogcapi-cli/data/ne_10m_admin_0_countries.geojson")?,
            collection: Some("countries".to_string()),
            ..Default::default()
        },
        &database_url,
    )
    .await?;

    import::ogr::load(
        Args {
            input: PathBuf::from_str("../ogcapi-cli/data/ne_10m_populated_places.geojson")?,
            collection: Some("places".to_string()),
            ..Default::default()
        },
        &database_url,
    )
    .await?;

    import::ogr::load(
        Args {
            input: PathBuf::from_str("../ogcapi-cli/data/ne_10m_railroads.geojson")?,
            collection: Some("railroads".to_string()),
            ..Default::default()
        },
        &database_url,
    )
    .await?;

    // query position
    let query = EdrQuery {
        coords: "POINT(2600000 1200000)".to_string(),
        parameter_name: Some("NAME,ISO_A2,CONTINENT".to_string()),
        crs: Crs::from(2056),
        ..Default::default()
    };

    let res = client
        .request(
            Request::builder()
                .method(axum::http::Method::GET)
                .uri(format!(
                    "http://{}/edr/countries/position?{}",
                    addr,
                    serde_qs::to_string(&query)?
                ))
                .body(hyper::Body::empty())
                .unwrap(),
        )
        .await?;

    assert_eq!(200, res.status());

    let body = hyper::body::to_bytes(res.into_body()).await?;
    let fc: FeatureCollection = serde_json::from_slice(&body)?;

    assert_eq!(fc.number_matched, Some(1));
    assert_eq!(fc.number_returned, Some(1));
    let feature = &fc.features[0];
    assert_eq!(feature.properties.as_ref().unwrap().0.len(), 3);
    assert_eq!(
        feature.properties.as_ref().unwrap().0["NAME"].as_str(),
        Some("Switzerland")
    );

    // query area
    let query = EdrQuery {
        coords: "POLYGON((7 46, 7 48, 9 48, 9 46, 7 46))".to_string(),
        parameter_name: Some("NAME,ISO_A2,ADM0NAME".to_string()),
        ..Default::default()
    };

    let res = client
        .request(
            Request::builder()
                .method(axum::http::Method::GET)
                .uri(format!(
                    "http://{}/edr/places/area?{}",
                    addr,
                    serde_qs::to_string(&query)?
                ))
                .body(hyper::Body::empty())
                .unwrap(),
        )
        .await?;

    assert_eq!(200, res.status());

    let body = hyper::body::to_bytes(res.into_body()).await?;
    let fc: FeatureCollection = serde_json::from_slice(&body)?;

    assert_eq!(fc.number_matched, Some(19));
    assert_eq!(fc.number_returned, Some(19));
    let feature = &fc
        .features
        .into_iter()
        .find(|f| f.properties.as_ref().unwrap().0["NAME"].as_str() == Some("Bern"));
    assert!(feature.is_some());

    // query radius
    let query = EdrQuery {
        coords: "POINT(7.5 47)".to_string(),
        parameter_name: Some("NAME,ISO_A2,ADM0NAME".to_string()),
        within: Some("1000".to_string()),
        within_units: Some("km".to_string()),
        ..Default::default()
    };

    let res = client
        .request(
            Request::builder()
                .method(axum::http::Method::GET)
                .uri(format!(
                    "http://{}/edr/countries/radius?{}",
                    addr,
                    serde_qs::to_string(&query)?
                ))
                .body(hyper::Body::empty())
                .unwrap(),
        )
        .await?;

    assert_eq!(200, res.status());

    let body = hyper::body::to_bytes(res.into_body()).await?;
    let mut fc: FeatureCollection = serde_json::from_slice(&body)?;

    for mut feature in fc.features.iter_mut() {
        feature.geometry = Json(Geometry::new(Value::Point(vec![0.0, 0.0])));
    }

    tracing::debug!("{}", serde_json::to_string_pretty(&fc.number_matched)?);
    // assert_eq!(features.number_matched, Some(19));
    // assert_eq!(features.number_returned, Some(19));
    // let feature = &features
    //     .features
    //     .into_iter()
    //     .find(|f| f.properties.as_ref().unwrap().0["NAME"].as_str() == Some("Bern"));
    // assert!(feature.is_some());

    Ok(())
}