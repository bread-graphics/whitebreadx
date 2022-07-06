# whitebreadx

[![crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Build Status][build-badge]][build-url]

[crates-badge]: https://img.shields.io/crates/v/whitebreadx
[crates-url]: https://crates.io/crates/whitebreadx
[docs-badge]: https://img.shields.io/docsrs/whitebreadx
[docs-url]: https://docs.rs/whitebreadx
[build-badge]: https://img.shields.io/github/workflow/status/bread-graphics/whitebreadx/CI
[build-url]: https://github.com/bread-graphics/whitebreadx/actions?query=workflow%3ACI+branch%3Amaster

Wrappers around `libxcb` and `libX11` that implement `breadx` traits.

`breadx` comes with many advantages over existing libraries, but 
a crucial disadvantage is a lack of library support. `libX11` has
a massive back catalog of libraries that `breadx` on its own does
not have access to.

`whitebreadx` provides a compromise. It provides two types, `XcbDisplay`
and `XlibDisplay`. Both of these objects are wrappers around native
`xcb_connection_t` and `Display`, respectively. However, they implement
`breadx::Display`, so that they can be used worry-free in `breadx` code.
In addition, raw pointers to the underlying transport mechanism can be
accessed, allowing usage with external code.

## External Library Version Support

The minimum supported versions of `libxcb` and `libX11` necessary for 
this library are unknown. This library has been tested to work with 
`libxcb` version 1.14 and `libX11` version 2:1.7. However, the `libX11` 
version must be after the paradigm shift where it began using `libxcb` 
as an internal transport. There are no plans to support legacy `libX11`.

## License

This package is distributed under the Boost Software License Version 1.0.
Consult the [LICENSE](./LICENSE) file or consult the [web mirror] for
more information.

[web mirror]: https://www.boost.org/LICENSE_1_0.txt
