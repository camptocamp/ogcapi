#![doc = include_str!("../README.md")]

#[cfg(feature = "import")]
pub mod import;

#[cfg(feature = "client")]
pub mod client {
    pub use ogcapi_client::*;
}

#[cfg(feature = "drivers")]
pub mod drivers {
    pub use ogcapi_drivers::*;
}

#[cfg(feature = "services")]
pub mod services {
    pub use ogcapi_services::*;
}

#[cfg(feature = "types")]
pub mod types {
    pub use ogcapi_types::*;
}
