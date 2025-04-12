use postgis::ewkb::{AsEwkbGeometry, EwkbRead, EwkbWrite, GeometryT, Point};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::types::Type;
use crate::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat, PgValueRef, Postgres};

// Define the PostgreSQL type information for geometry
impl Type<Postgres> for GeometryT<Point> {
    fn type_info() -> PgTypeInfo {
        // Use the PostGIS "geometry" type
        PgTypeInfo::with_name("geometry")
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        // The type is compatible if it's named "geometry"
        matches!(ty.name(), name if name == "geometry")
    }
}

// Implement array support
impl PgHasArrayType for GeometryT<Point> {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_geometry")
    }

    fn array_compatible(ty: &PgTypeInfo) -> bool {
        matches!(ty.name(), name if name == "_geometry")
    }
}

impl Encode<'_, Postgres> for GeometryT<Point> {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // PostGIS EWKB is already in the format expected by PostgreSQL, so we can
        // directly encode the binary representation
        self.as_ewkb().write_ewkb(&mut buf.as_mut_slice())?;
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        // A rough estimate based on geometry type - could be improved
        match self {
            GeometryT::Point(_) => 21,         // header (9) + point (8*1 + 4)  
            GeometryT::LineString(_) => 49,    // header (9) + typical 2-point line (2*8*2 + 8)
            GeometryT::Polygon(_) => 93,       // header (9) + typical 4-point polygon (4*8*2 + 24)
            GeometryT::MultiPoint(_) => 42,    // header (9) + 1 point (21) + overhead (12)
            GeometryT::MultiLineString(_) => 90, // header (9) + 1 linestring (49) + overhead (32)
            GeometryT::MultiPolygon(_) => 134,  // header (9) + 1 polygon (93) + overhead (32)
            GeometryT::GeometryCollection(_) => 32, // header (9) + minimal collection overhead (23)
            _ => 128,                          // generic fallback for unknown/custom types
        }
    }
}

impl<'db> Decode<'db, Postgres> for GeometryT<Point> {
    fn decode(value: PgValueRef<'db>) -> Result<Self, BoxDynError> {
        match value.format() {
            PgValueFormat::Binary => {
                // PostgreSQL returns geometry in EWKB format
                let mut bytes = value.as_bytes()?;
                
                // Use the postgis crate to parse EWKB
                GeometryT::<Point>::read_ewkb(&mut bytes)
                    .map_err(|e| format!("Failed to parse EWKB geometry: {}", e).into())
            }
            
            _ => Err(format!("Failed to parse WKT geometry").into())
        }
    }
}
