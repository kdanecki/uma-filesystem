mod bindings;
mod types;
use memmap2::*;
//use memmap2::MmapMut;
use std::{
    ffi::CStr,
    fs::{File, OpenOptions},
    io::Write,
};

use types::{superblock_t, Foo};

fn main() {
    let block_size = 1024;
    let block_num = 16348;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("foo")
        .expect("failed to open file");
    file.set_len(block_size * block_num).expect("OOM");
    let mut map = unsafe { MmapMut::map_mut(&file).expect("failed mmap") };

    let mut f = types::FileSystem::new(&mut map[..]);
    // println!("{:?}", f);
    f.format(1024, 16384);
    f.dummy_data();

    println!("{:?}", f.get_files_in_dir(c"/"));
    println!("{:?}", f.get_attr(c"/."));

    f.create_file(c"/blabla", &[], 0x8000 | 0o666);
    f.write_file(c"/blabla", &[1u8, 234, 65], 0);
    println!("{:?}", f.read_file(c"/blabla"));

    // f.rename(c"/blabla", c"/gogo").unwrap();
    // f.rename(c"/XD/xd", c"/ijvnbdv/gogo").unwrap();

    // f.test();

    //f.save();
    //println!("{:?}", f);
    /*map[0] = 123;
    map[2] = 255;
    (&mut map[3..6]).write(&[2, 4, 134]).expect("IO error");
    map[0..2].copy_from_slice(&[1, 2]);
    //println!("{:?}", map[..10]);
    let mut a = [1i8, 2, 3, 4, 5, 6, 7, 8];
    let mut sb = superblock_t::from(a);
    a[1] = 10;
    sb.header[3] = 123;
    println!("{:#?}", sb);
    println!("{:?}", a);

    let f = Foo { x: 1 };
    let sb = superblock_t::from(f);
    println!("{:#?}", sb);
    //    println!("{:?}", f);*/
    // let a: [u8; 28] = map[0..28].try_into().unwrap();
    // let mut sb: superblock_t = zerocopy::transmute!(a);
    // let b: [u8; 28] = zerocopy::transmute!(sb);
    //map[0..28].copy_from_slice(&b);
    // sb.header[0] = 0;

    // println!("{:#?}", sb);
    // println!("{:#?}", a);

    // let mut a = types::Goo::<10> { goo: [0; 10] };
    // let b = types::Goo { goo: [1; 5] };
    //let d = 10;
    //let c = types::Goo { goo: [1; d] };
    //a = b;
    //let b: () = a;
    /*    let boo: types::Boo = zerocopy::transmute!(sb);
    println!("{:#?}", boo)*/
}
