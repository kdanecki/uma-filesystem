all:
	cbindgen -o rust.h -l c src/lib.rs
	cargo build
	gcc main.c -lfs_rust -L./target/debug -D_FILE_OFFSET_BITS=64 -lfuse -pthread
	export LD_LIBRARY_PATH=./target/debug

mount:
	export LD_LIBRARY_PATH=./target/debug; ./a.out -d mp

umount:
	fusermount -u mp

prod:
	export LD_LIBRARY_PATH=./target/debug
	./a.out mp
