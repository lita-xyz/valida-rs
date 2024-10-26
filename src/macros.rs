#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const VALIDA_ENTRY: fn() = $path;

        mod valida_generated_main {
            use $crate::getrandom::register_custom_getrandom;
            use $crate::rand::valida_rand;

            register_custom_getrandom!(valida_rand);

            #[cfg_attr(not(test), no_mangle)]
            fn main() {
                super::VALIDA_ENTRY()
            }
        }
    };
}
