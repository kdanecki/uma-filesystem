# OxidizedFS

## Prerequirements

To compile you need rust compiler, cbindgen for generating C headers for rust crates and C compiler.

    $ apt install cargo gcc
    $ cargo install cbindgen

After cbindgen instalation you may get asked to add .cargo/bin to your PATH

    $ export PATH=/home/<username>/.cargo/bin:$PATH

## Build

Provided Makefile handles all compilation

    $ make

## Run

For detailed instructions consult help `./oxidisedFS -h`.
Makefile also has targets to automatically create image with default parameters and mount/unmount the fs.
