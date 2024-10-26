#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const DELENDUM_ENTRY: fn() = $path;

        mod delendum_generated_main {
            use $crate::getrandom::register_custom_getrandom;
            use $crate::rand::delendum_rand;

            register_custom_getrandom!(delendum_rand);

            #[cfg_attr(not(test), no_mangle)]
            fn main() {
                super::DELENDUM_ENTRY()
            }
        }
    };
}
