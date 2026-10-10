pub mod elevation;
pub mod registry;
pub mod shell;

pub use elevation::is_elevated;

pub use registry::{Hive, MockRegistry, RegValue, Registry, WinRegistry};
pub use shell::{MockShell, Output, RealShell, Shell};
