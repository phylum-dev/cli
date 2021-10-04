/// enables logging statically for any test module this module it is imported into
pub mod logging {

    use static_init::dynamic;

    #[dynamic]
    static mut _LOGGER_INIT: bool = {
        env_logger::init();
        true
    };
}
