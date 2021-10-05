//! This module is called mod.rs so when running tests, Cargo knows not to report this
//! as a test module, but treat it as module full of test utilities
//!
//! By simply importing or declaring this module, /tests test programs will have logging inited

use static_init::dynamic;

#[dynamic]
static mut _LOGGER_INIT: bool = {
    env_logger::init();
    true
};
