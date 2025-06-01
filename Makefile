EXEC_NAME=oxidizedFS

all:
	cbindgen -o rust.h -l c src/lib.rs
	cargo build
	gcc -o ${EXEC_NAME} -g main.c -lfs_rust -L./target/debug -D_FILE_OFFSET_BITS=64 -lfuse -pthread

create:
	export LD_LIBRARY_PATH=./target/debug; ./${EXEC_NAME} testImage format 1024 16384 8192

mount:
	mkdir mp
	export LD_LIBRARY_PATH=./target/debug;	./${EXEC_NAME} testImage mount mp
	
umount:
	fusermount -u mp
	rmdir mp
	
debug:
	export LD_LIBRARY_PATH=./target/debug; ./${EXEC_NAME} testImage mount -d mp

clean:
	cargo clean
	rm ${EXEC_NAME}
