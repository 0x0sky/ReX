pub use phf::Map;

#[doc(hidden)]
pub use phf::phf_map as __phf_map;

/// Compatibility macro for ReX's generated font tables.
///
/// The historical `static_map!` syntax is preserved while the implementation
/// delegates to PHF's stable compile-time perfect-hash generator. The legacy
/// `Default:` expression only filled unused Robin-Hood buckets and was never
/// observable through `Map::get`, so it is intentionally ignored here.
#[macro_export]
macro_rules! static_map {
    (Default: $default:expr, $($key:expr => $value:expr),* $(,)*) => {{
        $crate::__phf_map! {
            $($key => $value),*
        }
    }};
}
