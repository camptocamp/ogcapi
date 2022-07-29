# `ogcapi`

[OGC API](https://ogcapi.ogc.org/) building blocks implemented in [Rust](https://www.rust-lang.org/)

## Quick Start

This will take a while and use quite some disk space

```bash
# Setup the database
docker compose up

# Import administrative bounaries
docker exec -ti ogcapi cargo run -- import \
        --input data/ne_110m_admin_0_countries.geojson \
        --collection countries

# Run app
docker exec -ti ogcapi cargo run -- serve
```

Open <http://localhost:8484/> were you will find the `Landing Page`.

## Developing

### Prerequisites

- Rust
- Docker & Docker Compose
- GDAL

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres,rustls
```

### Setup

```bash
# Run services
docker compose up db minio minio-mc -d

# Import administrative bounaries
cargo run -- import --input data/ne_110m_admin_0_countries.geojson --collection countries

# Start service 
cargo run -- serve

# Run tests
cargo test --workspace --all-features

# Open Documentation
cargo doc --workspace --all-features --no-deps --open
```

### Format / Lint

```bash
# Format
cargo fmt

# Clippy
cargo clippy --workspace --all-features --tests
```

### Teamengine

```bash
docker run --network host ogccite/ets-ogcapi-features10
# docker run --network host ogccite/ets-ogcapi-edr10
```

Navigate to <http://localhost:8080/teamengine/> to execute the test suite. For documentation and more info see <https://cite.opengeospatial.org/teamengine/about/ogcapi-features-1.0/1.0/site>.

## Example Project

STAC enabled OGC API Features: <https://github.com/camptocamp/oapi-poc>

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
