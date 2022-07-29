use std::{fs, path::Path, str::FromStr};

#[doc(hidden)]
pub static OPENAPI: &[u8; 29696] = include_bytes!("../assets/openapi/openapi.yaml");

#[derive(Default, Clone)]
pub struct OpenAPI(pub openapiv3::OpenAPI);

impl FromStr for OpenAPI {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let openapi: openapiv3::OpenAPI = serde_yaml::from_str(s)?;
        Ok(OpenAPI(openapi))
    }
}

impl OpenAPI {
    pub fn from_slice(api: &[u8]) -> Self {
        let openapi: openapiv3::OpenAPI = serde_yaml::from_slice(api).unwrap();
        OpenAPI(openapi)
    }

    pub fn from_path(path: &Path) -> anyhow::Result<OpenAPI> {
        let api = fs::read_to_string(path)?;
        OpenAPI::from_str(&api)
    }
}
