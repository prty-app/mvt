// mapgrid.rs
//
// Copyright (c) 2019-2024  Minnesota Department of Transportation
//
//! TileId and MapGrid structs.
//!
use crate::error::{Error, Result};
use pointy::{BBox, Pt, Transform};
use std::fmt;

/// Half size of map (meters)
const HALF_SIZE_M: f64 = 20_037_508.342_789_248;

/// A tile ID identifies a tile on a map grid at a specific zoom level.
///
/// It uses XYZ addressing, with X increasing from west to east and Y increasing
/// from north to south.  The X and Y values can range from 0 to
/// 2<sup>Z</sup>-1.
#[derive(Clone, Copy, Debug)]
pub struct TileId {
    x: u32, // not public to prevent invalid values being created
    y: u32,
    z: u32,
}

/// A map grid is used to address [tile]s on a map.
///
/// The grid should be in projected coördinates.  Use `default()` for
/// [Web Mercator].
///
/// [tile]: struct.Tile.html
/// [Web Mercator]: https://en.wikipedia.org/wiki/Web_Mercator_projection
#[derive(Clone, Debug)]
pub struct MapGrid {
    /// Spatial reference ID
    srid: i32,

    /// Bounding box
    bbox: BBox<f64>,
}

impl TileId {
    /// Get the X value.
    pub fn x(&self) -> u32 {
        self.x
    }

    /// Get the Y value.
    pub fn y(&self) -> u32 {
        self.y
    }

    /// Get the Z (zoom) value.
    pub fn z(&self) -> u32 {
        self.z
    }
}

impl fmt::Display for TileId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}/{}", self.z, self.x, self.y)
    }
}

impl TileId {
    /// Create a new TildId.
    ///
    /// If invalid, returns [Error::InvalidTid](enum.Error.html).
    pub fn new(x: u32, y: u32, z: u32) -> Result<Self> {
        TileId::check_valid(x, y, z)?;
        Ok(TileId { x, y, z })
    }

    /// Check whether a tile ID is valid.
    fn check_valid(x: u32, y: u32, z: u32) -> Result<()> {
        if z > 31 {
            return Err(Error::InvalidTid());
        }
        let s = 1 << z;
        if x < s && y < s {
            Ok(())
        } else {
            Err(Error::InvalidTid())
        }
    }
}

impl Default for MapGrid {
    fn default() -> Self {
        const WEB_MERCATOR_SRID: i32 = 3857;
        let srid = WEB_MERCATOR_SRID;
        let p0 = Pt::new(-HALF_SIZE_M, -HALF_SIZE_M);
        let p1 = Pt::new(HALF_SIZE_M, HALF_SIZE_M);
        let bbox = BBox::from((p0, p1));
        Self { srid, bbox }
    }
}

impl MapGrid {
    /// Create a new map grid.
    ///
    /// * `srid` Spatial reference ID.
    /// * `bbox` Bounding box.
    pub fn new(srid: i32, bbox: BBox<f64>) -> Self {
        MapGrid { srid, bbox }
    }

    /// Get the spatial reference ID.
    pub fn srid(&self) -> i32 {
        self.srid
    }

    /// Get the bounding box of the grid.
    pub fn bbox(&self) -> BBox<f64> {
        self.bbox
    }

    /// Get the bounding box of a tile ID.
    pub fn tile_bbox(&self, tid: TileId) -> BBox<f64> {
        let tx = self.bbox.x_min(); // west edge
        let ty = self.bbox.y_max(); // north edge
        let tz = zoom_scale(tid.z);
        let sx = self.bbox.x_span() * tz;
        let sy = self.bbox.y_span() * tz;
        let t = Transform::with_scale(sx, -sy).translate(tx, ty);
        let tidx = f64::from(tid.x);
        let tidy = f64::from(tid.y);
        let p0 = t * Pt::new(tidx, tidy);
        let p1 = t * Pt::new(tidx + 1.0, tidy + 1.0);
        BBox::from((p0, p1))
    }

    /// Get the transform to coördinates in 0 to 1 range.
    pub fn tile_transform(&self, tid: TileId) -> Transform<f64> {
        let tx = self.bbox.x_min(); // west edge
        let ty = self.bbox.y_max(); // north edge
        let tz = f64::from(1 << tid.z);
        let sx = tz / self.bbox.x_span();
        let sy = tz / self.bbox.y_span();
        let tidx = f64::from(tid.x);
        let tidy = f64::from(tid.y);
        Transform::with_translate(-tx, -ty)
            .scale(sx, -sy)
            .translate(-tidx, -tidy)
    }
}

/// Calculate scales at one zoom level.
fn zoom_scale(zoom: u32) -> f64 {
    1.0 / f64::from(1 << zoom)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tile_bbox() {
        let g = MapGrid::<f64>::default();
        let tid = TileId::new(0, 0, 0).unwrap();
        let b = g.tile_bbox(tid);
        assert_eq!(b.x_min(), -20037508.3427892480);
        assert_eq!(b.x_max(), 20037508.3427892480);
        assert_eq!(b.y_min(), -20037508.3427892480);
        assert_eq!(b.y_max(), 20037508.3427892480);

        let tid = TileId::new(0, 0, 1).unwrap();
        let b = g.tile_bbox(tid);
        assert_eq!(b.x_min(), -20037508.3427892480);
        assert_eq!(b.x_max(), 0.0);
        assert_eq!(b.y_min(), 0.0);
        assert_eq!(b.y_max(), 20037508.3427892480);

        let tid = TileId::new(1, 1, 1).unwrap();
        let b = g.tile_bbox(tid);
        assert_eq!(b.x_min(), 0.0);
        assert_eq!(b.x_max(), 20037508.3427892480);
        assert_eq!(b.y_min(), -20037508.3427892480);
        assert_eq!(b.y_max(), 0.0);

        let tid = TileId::new(246, 368, 10).unwrap();
        let b = g.tile_bbox(tid);
        assert_eq!(b.x_min(), -10410111.756214727);
        assert_eq!(b.x_max(), -10370975.997732716);
        assert_eq!(b.y_min(), 5596413.462927466);
        assert_eq!(b.y_max(), 5635549.221409475);
    }

    #[test]
    fn test_tile_transform() {
        let g = MapGrid::default();
        let tid = TileId::new(0, 0, 0).unwrap();
        let t = g.tile_transform(tid);
        assert_eq!(
            Pt::new(0.0, 0.0),
            t * Pt::new(-20037508.3427892480, 20037508.3427892480)
        );
        assert_eq!(
            Pt::new(1.0, 1.0),
            t * Pt::new(20037508.3427892480, -20037508.3427892480)
        );

        let tid = TileId::new(0, 0, 1).unwrap();
        let t = g.tile_transform(tid);
        assert_eq!(
            Pt::new(0.0, 0.0),
            t * Pt::new(-20037508.3427892480, 20037508.3427892480)
        );
        assert_eq!(Pt::new(1.0, 1.0), t * Pt::new(0.0, 0.0));

        let tid = TileId::new(1, 1, 1).unwrap();
        let t = g.tile_transform(tid);
        assert_eq!(Pt::new(0.0, 0.0), t * Pt::new(0.0, 0.0));
        assert_eq!(
            Pt::new(1.0, 1.0),
            t * Pt::new(20037508.3427892480, -20037508.3427892480)
        );

        let tid = TileId::new(246, 368, 10).unwrap();
        let t = g.tile_transform(tid);
        assert_eq!(
            Pt::new(0.0, 0.0),
            t * Pt::new(-10410111.756214727, 5635549.221409475)
        );
        assert_eq!(
            Pt::new(1.0, 0.9999999999999716),
            t * Pt::new(-10370975.997732716, 5596413.462927466)
        );
    }
}
