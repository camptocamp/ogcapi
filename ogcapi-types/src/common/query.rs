use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::common::{Bbox, Crs, Datetime};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Query {
    #[serde(default)]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub bbox: Option<Bbox>,
    #[serde(default)]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub bbox_crs: Option<Crs>,
    #[serde(default)]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub datetime: Option<Datetime>,
    pub limit: Option<isize>,
    pub offset: Option<isize>,
    pub f: Option<String>,
}
