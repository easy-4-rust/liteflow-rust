//! 脚本子系统专用异常。

pub mod script_load_exception;
pub mod script_spi_exception;

pub use script_load_exception::ScriptLoadException;
pub use script_spi_exception::ScriptSpiException;
