use std::num::NonZeroUsize;

use crate::cpl::CslStringList;
use crate::errors;
use crate::raster::processing::dem::options::common_dem_options;
use crate::raster::processing::dem::DemSlopeAlg;

/// Configuration options for [`slope()`][super::slope()].
#[derive(Debug, Clone, Default)]
pub struct SlopeOptions {
    input_band: Option<NonZeroUsize>,
    compute_edges: bool,
    output_format: Option<String>,
    additional_options: CslStringList,
    algorithm: Option<DemSlopeAlg>,
    scale: Option<f64>,
    percentage_results: Option<bool>,
}

impl SlopeOptions {
    /// Create a DEM-slope options set.
    pub fn new() -> Self {
        Default::default()
    }

    common_dem_options!();

    /// Specify the slope computation algorithm.
    pub fn with_algorithm(&mut self, algorithm: DemSlopeAlg) -> &mut Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// Fetch the specified slope computation algorithm.
    pub fn algorithm(&self) -> Option<DemSlopeAlg> {
        self.algorithm
    }

    /// Apply a elevation scaling factor.
    ///
    /// Routine assumes x, y and z units are identical.
    /// If x (east-west) and y (north-south) units are identical, but z (elevation) units are different,
    /// this scale option can be used to set the ratio of vertical units to horizontal.
    ///
    /// For LatLong projections <u>near the equator</u>, where units of latitude and units of longitude are
    /// similar, elevation (z) units can be converted with the following values:
    ///
    /// * Elevation in feet: `370400`
    /// * Elevation in meters: `111120`
    ///
    /// For locations not near the equator, it would be best to reproject your raster first.
    pub fn with_scale(&mut self, scale: f64) -> &mut Self {
        self.scale = Some(scale);
        self
    }

    /// Fetch the specified scaling factor.
    ///
    /// Returns `None` if one has not been previously set via [`Self::with_scale`].
    pub fn scale(&self) -> Option<f64> {
        self.scale
    }

    /// If `state` is `true`, the slope will be expressed as percent slope.
    ///
    /// Otherwise, it is expressed as degrees
    pub fn with_percentage_results(&mut self, state: bool) -> &mut Self {
        self.percentage_results = Some(state);
        self
    }

    /// Render relevant common options into [`CslStringList`] values, as compatible with
    /// [`gdal_sys::GDALDEMProcessing`].
    pub fn to_options_list(&self) -> errors::Result<CslStringList> {
        let mut opts = CslStringList::default();

        self.store_common_options_to(&mut opts)?;

        if let Some(alg) = self.algorithm {
            opts.add_string("-alg")?;
            opts.add_string(alg.to_gdal_option())?;
        }

        if let Some(scale) = self.scale {
            opts.add_string("-s")?;
            opts.add_string(&scale.to_string())?;
        }

        if self.percentage_results == Some(true) {
            opts.add_string("-p")?;
        }

        Ok(opts)
    }
}

#[cfg(test)]
mod tests {
    use crate::cpl::CslStringList;
    use crate::errors::Result;
    use crate::raster::processing::dem::slope;
    use crate::raster::StatisticsAll;
    use crate::test_utils::{fixture, target};
    use crate::Dataset;
    use crate::{assert_near, GeoTransformEx};

    use super::*;

    #[test]
    fn test_options() -> Result<()> {
        let mut proc = SlopeOptions::new();
        proc.with_input_band(2.try_into().unwrap())
            .with_algorithm(DemSlopeAlg::ZevenbergenThorne)
            .with_scale(98473.0)
            .with_compute_edges(true)
            .with_percentage_results(true)
            .with_output_format("GTiff")
            .with_additional_options("CPL_DEBUG=ON".parse()?);

        let expected: CslStringList =
            "-compute_edges -b 2 -of GTiff CPL_DEBUG=ON -alg ZevenbergenThorne -s 98473 -p"
                .parse()?;
        assert_eq!(expected.to_string(), proc.to_options_list()?.to_string());

        Ok(())
    }

    #[test]
    fn test_slope() -> Result<()> {
        // For X & Y in degrees and Z in meters...
        fn scaling_estimate(ds: &Dataset) -> f64 {
            let (_, lat) = ds.geo_transform().unwrap().apply(0., 0.);
            0.5 * (111120.0 + lat.to_radians().cos() * 111120.0)
        }

        let ds = Dataset::open(fixture("dem-hills.tiff"))?;
        let scale_factor = scaling_estimate(&ds);

        let mut opts = SlopeOptions::new();
        opts.with_algorithm(DemSlopeAlg::Horn)
            .with_percentage_results(true)
            .with_scale(scale_factor);

        let slope = slope(&ds, target("dem-hills-slope.tiff"), &opts)?;

        let stats = slope.rasterband(1)?.get_statistics(true, false)?.unwrap();

        // These numbers were generated by extracting the output from:
        //    gdaldem slope -alg Horn -s 98473.2947 -p fixtures/dem-hills.tiff target/dest.tiff
        //    gdalinfo -stats target/dest.tiff
        let expected = StatisticsAll {
            min: 0.0,
            max: 65.440422058105,
            mean: 6.1710967990449,
            std_dev: 8.73558602352,
        };

        assert_near!(StatisticsAll, stats, expected, epsilon = 1e-8);
        Ok(())
    }
}
