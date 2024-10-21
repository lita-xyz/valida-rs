#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const DELENDUM_ENTRY: fn() = $path;

        mod delendum_generated_main {
            use $crate::rand::delendum_rand;
            use $crate::getrandom::register_custom_getrandom;

            register_custom_getrandom!(delendum_rand);

            #[no_mangle]
            fn main() {
                super::DELENDUM_ENTRY()
            }
        }
    };
}
