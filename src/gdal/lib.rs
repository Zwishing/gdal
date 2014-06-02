#![crate_id="gdal#0.1"]
#![crate_type="lib"]

extern crate sync;
extern crate libc;
#[cfg(test)] extern crate test;

use std::str::raw;
use libc::c_char;
use sync::mutex::{StaticMutex, MUTEX_INIT};

pub mod driver;
pub mod raster;
pub mod proj;
pub mod geom;
pub mod warp;


#[link(name="gdal")]
extern {
    fn GDALVersionInfo(key: *c_char) -> *c_char;
    fn GDALAllRegister();
}


static mut LOCK: StaticMutex = MUTEX_INIT;
static mut registered_drivers: bool = false;

fn register_drivers() {
    unsafe {
        let _g = LOCK.lock();
        if ! registered_drivers {
            GDALAllRegister();
            registered_drivers = true;
        }
    }
}


pub fn version_info(key: &str) -> String {
    let info = key.with_c_str(|c_key| {
        unsafe {
            let rv = GDALVersionInfo(c_key);
            return raw::from_c_str(rv);
        };
    });
    return info;
}


#[test]
fn test_version_info() {
    let release_date = version_info("RELEASE_DATE");
    let release_name = version_info("RELEASE_NAME");
    let version_text = version_info("--version");

    let expected_text: String = format!(
        "GDAL {}, released {}/{}/{}",
        release_name,
        release_date.as_slice().slice(0, 4),
        release_date.as_slice().slice(4, 6),
        release_date.as_slice().slice(6, 8),
    );

    assert_eq!(version_text.into_owned(), expected_text);
}