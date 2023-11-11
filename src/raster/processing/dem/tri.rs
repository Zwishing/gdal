use std::num::NonZeroUsize;

use crate::cpl::CslStringList;
use crate::errors;
use crate::raster::processing::dem::options::common_dem_options;

/// Configuration options for [`terrain_ruggedness_index()`][super::terrain_ruggedness_index()].
#[derive(Debug, Clone, Default)]
pub struct TriOptions {
    input_band: Option<NonZeroUsize>,
    compute_edges: bool,
    output_format: Option<String>,
    additional_options: CslStringList,
    algorithm: Option<DemTriAlg>,
}

impl TriOptions {
    /// Create a DEM-terrain-ruggedness-index options set.
    pub fn new() -> Self {
        Default::default()
    }

    common_dem_options!();

    /// Specify the slope computation algorithm.
    pub fn with_algorithm(&mut self, algorithm: DemTriAlg) -> &mut Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// Render relevant common options into [`CslStringList`] values, as compatible with
    /// [`gdal_sys::GDALDEMProcessing`].
    pub fn to_options_list(&self) -> errors::Result<CslStringList> {
        let mut opts = CslStringList::default();

        self.store_common_options_to(&mut opts)?;

        // Before 3.3, Wilson is the only algorithm and therefore there's no
        // selection option. Rust caller can still specify Wilson, but
        // we don't pass it along.
        #[cfg(all(major_is_3, minor_ge_3))]
        if let Some(alg) = self.algorithm {
            opts.add_string("-alg")?;
            opts.add_string(alg.to_gdal_option())?;
        }

        Ok(opts)
    }
}

/// Algorithm for computing Terrain Ruggedness Index (TRI).
#[derive(Debug, Clone, Copy)]
pub enum DemTriAlg {
    /// The Wilson (see Wilson et al 2007, Marine Geodesy 30:3-35) algorithm uses the mean
    /// difference between a central pixel and its surrounding cells.
    /// This is recommended for bathymetric use cases.
    Wilson,
    #[cfg(all(major_is_3, minor_ge_3))]
    /// The Riley algorithm (see Riley, S.J., De Gloria, S.D., Elliot, R. (1999):
    /// A Terrain Ruggedness that Quantifies Topographic Heterogeneity.
    /// Intermountain Journal of Science, Vol.5, No.1-4, pp.23-27) uses the square root of the
    /// sum of the square of the difference between a central pixel and its surrounding cells.
    /// This is recommended for terrestrial use cases.
    ///
    /// Only available in GDAL >= 3.3
    Riley,
}

impl DemTriAlg {
    pub(crate) fn to_gdal_option(&self) -> &'static str {
        match self {
            DemTriAlg::Wilson => "Wilson",
            #[cfg(all(major_is_3, minor_ge_3))]
            DemTriAlg::Riley => "Riley",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_near;
    use crate::errors::Result;
    use crate::raster::processing::dem::terrain_ruggedness_index;
    use crate::raster::StatisticsAll;
    use crate::test_utils::{fixture, target};
    use crate::Dataset;

    use super::*;

    #[cfg(all(major_is_3, minor_ge_3))]
    #[test]
    fn test_options() -> Result<()> {
        use crate::cpl::CslStringList;
        let mut opts = TriOptions::new();
        opts.with_input_band(2.try_into().unwrap())
            .with_compute_edges(true)
            .with_algorithm(DemTriAlg::Wilson)
            .with_output_format("GTiff")
            .with_additional_options("CPL_DEBUG=ON".parse()?);

        let expected: CslStringList =
            "-compute_edges -b 2 -of GTiff CPL_DEBUG=ON -alg Wilson".parse()?;
        assert_eq!(expected.to_string(), opts.to_options_list()?.to_string());

        Ok(())
    }

    #[test]
    fn test_tri() -> Result<()> {
        let mut opts = TriOptions::new();
        opts.with_algorithm(DemTriAlg::Wilson);

        let ds = Dataset::open(fixture("dem-hills.tiff"))?;

        let tri = terrain_ruggedness_index(&ds, target("dem-hills-tri.tiff"), &opts)?;

        let stats = tri.rasterband(1)?.get_statistics(true, false)?.unwrap();

        // These numbers were generated by extracting the output from:
        //    gdaldem tri -alg Wilson fixtures/dem-hills.tiff target/dest.tiff
        //    gdalinfo -stats target/dest.tiff
        let expected = StatisticsAll {
            min: 0.0,
            max: 4.9836235046387,
            mean: 0.49063101456532,
            std_dev: 0.67193563366948,
        };

        assert_near!(StatisticsAll, stats, expected, epsilon = 1e-10);
        Ok(())
    }
}
