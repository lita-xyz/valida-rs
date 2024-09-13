#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const DELENDUM_ENTRY: fn() = $path;

        mod delendum_generated_main {
            use entrypoint::rand::delendum_rand;
            use getrandom::register_custom_getrandom;

            register_custom_getrandom!(delendum_rand);

            #[no_mangle]
            fn main() {
                super::DELENDUM_ENTRY()
            }
        }
    };
}
