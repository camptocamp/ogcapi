use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::common::{Crs, Links};

use super::{BoundingBox2D, Point2D, TitleDescriptionKeywords};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileMatrixSets {
    pub tile_matrix_sets: Vec<TileMatrixSetItem>,
}

/// A minimal tile matrix set element for use within a list of tile matrix
/// sets linking to a full definition.
#[serde_as]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TileMatrixSetItem {
    /// Optional local tile matrix set identifier, e.g. for use as unspecified
    /// `{tileMatrixSetId}` parameter. Implementation of 'identifier'
    pub id: Option<String>,
    /// Title of this tile matrix set, normally used for display to a human
    pub title: Option<String>,
    /// Reference to an official source for this tileMatrixSet
    pub uri: Option<String>,
    #[serde(default)]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub crs: Option<Crs>,
    /// Links to related resources. A 'self' link to the tile matrix set definition is required.
    pub links: Links,
}

/// A definition of a tile matrix set following the Tile Matrix Set standard.
/// For tileset metadata, such a description (in `tileMatrixSet` property) is
/// only required for offline use, as an alternative to a link with a
/// `http://www.opengis.net/def/rel/ogc/1.0/tiling-scheme` relation type.
#[serde_as]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TileMatrixSet {
    #[serde(flatten)]
    pub title_description_keywords: TitleDescriptionKeywords,
    /// Tile matrix set identifier. Implementation of 'identifier'
    pub id: String,
    /// Reference to an official source for this TileMatrixSet
    pub uri: Option<String>,
    /// Coordinate Reference System (CRS)
    #[serde_as(as = "DisplayFromStr")]
    pub crs: Crs,
    pub ordered_axes: Option<Vec<String>>,
    /// Reference to a well-known scale set
    pub well_known_scale_set: Option<String>,
    /// Minimum bounding rectangle surrounding the tile matrix set, in the
    /// supported CRS
    pub bounding_box: Option<BoundingBox2D>,
    /// Describes scale levels and its tile matrices
    pub tile_matrices: Vec<TileMatrix>,
}

#[serde_as]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TileMatrix {
    #[serde(flatten)]
    pub title_description_keywords: TitleDescriptionKeywords,
    /// Identifier selecting one of the scales defined in the [TileMatrixSet]
    /// and representing the scaleDenominator the tile. Implementation of 'identifier'
    pub id: String,
    /// Scale denominator of this tile matrix
    pub scale_denominator: f64,
    /// Cell size of this tile matrix
    pub cell_size: f64,
    /// description": "The corner of the tile matrix (_topLeft_ or
    /// _bottomLeft_) used as the origin for numbering tile rows and columns.
    /// This corner is also a corner of the (0, 0) tile.
    #[serde(default)]
    pub corner_of_origin: CornerOfOrigin,
    /// Precise position in CRS coordinates of the corner of origin (e.g. the
    /// top-left corner) for this tile matrix. This position is also a corner
    /// of the (0, 0) tile. In previous version, this was 'topLeftCorner' and
    /// 'cornerOfOrigin' did not exist.
    pub point_of_origin: Point2D,
    /// Width of each tile of this tile matrix in pixels
    pub tile_width: NonZeroU64,
    /// Height of each tile of this tile matrix in pixels
    pub tile_height: NonZeroU64,
    /// Width of the matrix (number of tiles in width)
    pub matrix_width: NonZeroU64,
    /// Height of the matrix (number of tiles in height)
    pub matrix_height: NonZeroU64,
    /// Describes the rows that has variable matrix width
    pub variable_matrix_widths: Option<Vec<VariableMatrixWidth>>,
}

/// Variable Matrix Width data structure
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VariableMatrixWidth {
    /// Number of tiles in width that coalesce in a single tile for these rows
    pub coalesc: NonZeroU64,
    /// First tile row where the coalescence factor applies for this tilematrix
    pub min_tile_row: u64,
    /// Last tile row where the coalescence factor applies for this tilematrix
    pub smax_tile_row: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CornerOfOrigin {
    TopLeft,
    BottomLeft,
}

impl Default for CornerOfOrigin {
    fn default() -> Self {
        CornerOfOrigin::TopLeft
    }
}

#[cfg(test)]
mod test {
    use super::TileMatrixSet;

    #[test]
    fn parse_tms_example() {
        let content =
            std::fs::read_to_string("../ogcapi-services/assets/tms/WebMercartorQuad.json").unwrap();
        let tms: TileMatrixSet = serde_json::from_str(&content).unwrap();
        dbg!(&tms);
        println!("{}", serde_json::to_string_pretty(&tms).unwrap());
    }
}
