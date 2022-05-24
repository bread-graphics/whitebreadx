# whitebreadx

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

## License

MIT/Apache2