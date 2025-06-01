use std::{
    ffi::CStr,
    time::{SystemTime, UNIX_EPOCH},
};

use zerocopy::FromZeros;

#[derive(Debug)]
pub struct FileSystem<'a> {
    sb: superblock_t,
    data: &'a mut [u8],
    inode_bitmap: Bitmap<'a>,
    inodes: &'a mut [u8],
    blocks_bitmap: Bitmap<'a>,
    data_blocks: &'a mut [u8],
}

impl<'a> FileSystem<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        let sb_data: [u8; 28] = data[0..28].try_into().unwrap();
        let sb: superblock_t = zerocopy::transmute!(sb_data);

        let inode_bitmap_id = 1;
        let inodes_id = if sb.inodes_num % (8 * sb.block_size) == 0 {
            sb.inodes_num / (8 * sb.block_size) + inode_bitmap_id
        } else {
            sb.inodes_num / (8 * sb.block_size) + 2
        };

        let blocks_bitmap_id = if (sb.inodes_num * 128) % (sb.block_size) == 0 {
            (sb.inodes_num * 128) / (sb.block_size) + inodes_id
        } else {
            (sb.inodes_num * 128) / (sb.block_size) + inodes_id + 1
        };

        let data_blocks = sb.blocks_num - blocks_bitmap_id;
        let first_block_id = if data_blocks % (8 * sb.block_size) == 0 {
            data_blocks / (8 * sb.block_size) + blocks_bitmap_id
        } else {
            data_blocks / (8 * sb.block_size) + blocks_bitmap_id + 1
        };

        let (data, blocks_data) = data.split_at_mut((first_block_id * sb.block_size) as usize);
        let (data, blocks_bitmap_data) =
            data.split_at_mut((blocks_bitmap_id * sb.block_size) as usize);
        let (data, inodes_data) = data.split_at_mut((inodes_id * sb.block_size) as usize);
        let (data, inodes_bitmap_data) = data.split_at_mut(sb.block_size as usize);
        let inode_bitmap = Bitmap::new(inodes_bitmap_data, sb.inodes_num as usize);
        let blocks_bitmap = Bitmap::new(
            blocks_bitmap_data,
            (sb.blocks_num - first_block_id) as usize,
        );
        // println!(
        //     "{inodes_id} {blocks_bitmap_id} {first_block_id} {}",
        //     sb.blocks_num - first_block_id
        // );

        Self {
            sb,
            data,
            inode_bitmap,
            inodes: inodes_data,
            blocks_bitmap,
            data_blocks: blocks_data,
        }
    }

    pub fn format(&mut self) {
        self.sb.header = [0x58, 0x44, 0x20, 0x20, 0x20, 0x20, 0x58, 0x44];
        self.save();
        self.create_inode(1, 1, 0, 0x4000 | 0o755);

        let inode_num = 1;
        let mut data = [0u8; 22];
        data[..4].copy_from_slice(&(inode_num as u32).to_le_bytes());
        data[4..8].copy_from_slice(&1u32.to_le_bytes());
        data[8..9].copy_from_slice(".".as_bytes());

        data[12..16].copy_from_slice(&(inode_num as u32).to_le_bytes());
        data[16..20].copy_from_slice(&2u32.to_le_bytes());
        data[20..22].copy_from_slice("..".as_bytes());
        self.get_data_block_mut(1 as u32)[0..data.len()].copy_from_slice(&data);
        self.inode_bitmap.take(1);
        self.blocks_bitmap.take(1);
    }

    pub fn rename(&mut self, from: &CStr, to: &CStr) -> Result<(), &str> {
        let from = from.to_str().expect("path should be UTF-8");
        let to = to.to_str().expect("path should be UTF-8");
        if let Some(offset) = from.rfind('/') {
            if let Some(dir_from) = if offset == 0 {
                Some(self.get_inode_by_id(1))
            } else {
                self.find_file(&from[..offset])
            } {
                if dir_from.is_directory() {
                    if let Some(id) = self.search_directory_get_id(&dir_from, &from[offset + 1..]) {
                        if let Some(to_remove) = self.find_file(to) {
                            if to_remove.is_directory() {
                                return Err("file already exists");
                            }
                            self.unlink_file(to)?;
                        }
                        if let Some(to_offset) = to.rfind('/') {
                            if let Some((dir_to, node_id)) = if to_offset == 0 {
                                Some((self.get_inode_by_id(1), 1))
                            } else {
                                self.find_file_mut(&to[..to_offset])
                            } {
                                self.clear_dentry(&dir_from, &from[offset + 1..]);

                                // create dentry
                                let name = &to[to_offset + 1..].as_bytes();
                                self.create_dentry(&dir_to, node_id, id, name);
                                return Ok(());
                            }
                            return Err("target dir nonexisting");
                        }
                    }
                }
            }
            return Err("file not found");
        } else {
            return Err("bad filename format");
        }
    }

    pub fn save(&mut self) {
        let d: [u8; 28] = zerocopy::transmute!(self.sb);
        self.data[..28].copy_from_slice(&d);
    }

    pub fn get_attr(&self, path: &CStr) -> Option<inode_t> {
        self.find_file(path.to_str().unwrap())
    }

    pub fn create_file(&mut self, path: &CStr, content: &[u8], mode: u32) -> Result<(), &str> {
        return self.create_file_inter(path, content, mode as u16);
    }

    pub fn unlink_file(&mut self, path: &str) -> Result<(), &'static str> {
        // TODO check nlink
        // let path = path.to_str().unwrap();
        if let Some(offset) = path.rfind('/') {
            if let Some(node) = if offset == 0 {
                Some(self.get_inode_by_id(1))
            } else {
                self.find_file(&path[..offset])
            } {
                if node.is_directory() {
                    if let Some(id) = self.search_directory_get_id(&node, &path[offset + 1..]) {
                        let file = self.get_inode_by_id(id);
                        println!("{file:?}");
                        self.truncate_inter(file, id, 0).unwrap();
                        self.inode_bitmap.free(id as usize);
                        self.clear_dentry(&node, &path[offset + 1..]);

                        return Ok(());
                    }
                }
            }
            return Err("file not found");
        } else {
            return Err("bad filename format");
        }
    }

    pub fn unlink_dir(&mut self, path: &CStr) -> Result<(), &str> {
        let path = path.to_str().unwrap();
        if let Some(offset) = path.rfind('/') {
            if let Some(node) = if offset == 0 {
                Some(self.get_inode_by_id(1))
            } else {
                self.find_file(&path[..offset])
            } {
                if node.is_directory() {
                    if let Some(id) = self.search_directory_get_id(&node, &path[offset + 1..]) {
                        let file = self.get_inode_by_id(id);
                        if file.is_directory() {
                            let all_data = self.get_dir_data(&file);
                            let mut data = &all_data[..];
                            while let Some(d) = Dentry::from(&data[..]) {
                                println!("{:?}", d);
                                if !(d.name == "." || d.name == "..") {
                                    return Err("directory not empty");
                                }
                                data = &data[d.size..];
                            }
                            self.truncate_inter(file, id, 0).unwrap();
                            self.inode_bitmap.free(id as usize);
                            self.clear_dentry(&node, &path[offset + 1..]);

                            return Ok(());
                        }
                        return Err("not a directory");
                    }
                }
            }
            return Err("file not found");
        }
        Err("bad filename format")
    }

    fn find_file(&self, path: &str) -> Option<inode_t> {
        let root = self.get_inode_by_id(1);
        if path == "/" {
            return Some(root);
        }
        if &path[0..1] == "/" {
            return self.find_file_inter(&root, &path[1..]);
        }
        None
    }

    fn find_file_inter(&self, node: &inode_t, path: &str) -> Option<inode_t> {
        if let Some(offset) = path.find('/') {
            let filename = &path[0..offset];
            // println!("{:?}", filename.as_bytes());
            if let Some(sub_node) = self.search_directory(node, filename) {
                if sub_node.is_directory() {
                    // println!(
                    //     "subnode foid off {}searching {}",
                    //     offset,
                    //     &path[offset + 1..]
                    // );
                    return self.find_file_inter(&sub_node, &path[offset + 1..]);
                }
            } else {
                return None;
            }
        } else {
            return self.search_directory(node, path);
        };

        None
    }

    fn find_file_mut(&self, path: &str) -> Option<(inode_t, inode_p)> {
        let root = self.get_inode_by_id(1);
        if path == "/" {
            return Some((root, 1));
        }
        if &path[0..1] == "/" {
            return self.find_file_mut_inter(&root, &path[1..]);
        }
        None
    }

    fn find_file_mut_inter(&self, node: &inode_t, path: &str) -> Option<(inode_t, inode_p)> {
        if let Some(offset) = path.find('/') {
            let filename = &path[0..offset];
            // println!("{:?}", filename.as_bytes());
            if let Some(sub_node) = self.search_directory(node, filename) {
                if sub_node.is_directory() {
                    // println!(
                    //     "subnode foid off {}searching {}",
                    //     offset,
                    //     &path[offset + 1..]
                    // );
                    return self.find_file_mut_inter(&sub_node, &path[offset + 1..]);
                }
            } else {
                return None;
            }
        } else {
            if let Some(id) = self.search_directory_get_id(node, path) {
                return Some((self.get_inode_by_id(id), id));
            }
        };

        None
    }

    pub fn create_directory(&mut self, path: &CStr) -> Result<(), &str> {
        let data = vec![0; self.sb.block_size as usize];
        self.create_file_inter(path, &data, 0x4000 | 0o755)
    }

    fn write_to_indirect_block(
        &mut self,
        ind_block_num: u32,
        content: &[u8],
        offset: usize,
    ) -> Result<usize, &'static str> {
        let bs = self.sb.block_size as usize;
        let total = content.len();
        let mut block_num = offset / bs;
        let mut content = content;

        let start = offset % bs;
        let batch = if content.len() < bs - start {
            content.len()
        } else {
            bs - start
        };
        let indirect_data = self.get_data_block(ind_block_num);
        // println!("{ind_block_num}");
        // println!("{indirect_data:?}");
        let b = u32::from_le_bytes(
            indirect_data[block_num * 4..block_num * 4 + 4]
                .try_into()
                .unwrap(),
        );
        if b == 0 {
            return Err("attemted to write to block 0");
        }
        self.get_data_block_mut(b)[start..start + batch].copy_from_slice(&content[..batch]);
        content = &content[batch..];
        block_num += 1;
        while content.len() > 0 {
            if block_num < bs / 4 {
                let batch = if content.len() < bs {
                    content.len() as usize
                } else {
                    bs
                };
                let indirect_data = self.get_data_block(ind_block_num);
                let b = u32::from_le_bytes(
                    indirect_data[block_num * 4..block_num * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                if b == 0 {
                    return Err("attemted to write to block 0");
                }
                self.get_data_block_mut(b)[..batch].copy_from_slice(&content[..batch]);
                content = &content[batch..];
                block_num += 1;
            } else {
                return Ok(total - content.len());
            }
        }
        Ok(total)
    }

    fn write_to_double_indirect_block(
        &mut self,
        dob_block_num: u32,
        content: &[u8],
        offset: usize,
    ) -> Result<usize, &'static str> {
        let bs = (self.sb.block_size * self.sb.block_size / 4) as usize;
        let total = content.len();
        let mut block_num = offset / bs;
        let mut content = content;

        let start = offset % bs;
        let batch = if content.len() < bs - start {
            content.len()
        } else {
            bs - start
        };
        let indirect_data = self.get_data_block(dob_block_num);
        // println!("{dob_block_num}");
        // println!("doubly {indirect_data:?}, {offset}, con {}", content.len());

        let b = u32::from_le_bytes(
            indirect_data[block_num * 4..block_num * 4 + 4]
                .try_into()
                .unwrap(),
        );
        // println!(" b{b}");
        if b == 0 {
            return Err("attemted to write to block 0");
        }
        // self.get_data_block_mut(b)[start..start + batch].copy_from_slice(&content[..batch]);
        self.write_to_indirect_block(b, &content[..batch], start)?;
        content = &content[batch..];
        block_num += 1;
        while content.len() > 0 {
            if block_num < self.sb.block_size as usize / 4 {
                let batch = if content.len() < bs {
                    content.len() as usize
                } else {
                    bs
                };
                let indirect_data = self.get_data_block(dob_block_num);
                let b = u32::from_le_bytes(
                    indirect_data[block_num * 4..block_num * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                if b == 0 {
                    return Err("attemted to write to block 0");
                }
                // self.get_data_block_mut(b)[..batch].copy_from_slice(&content[..batch]);
                self.write_to_indirect_block(b, &content[..batch], 0)?;
                content = &content[batch..];
                block_num += 1;
            } else {
                return Ok(total - content.len());
            }
        }
        Ok(total)
    }

    fn write_file_data(
        &mut self,
        node: &inode_t,
        content: &[u8],
        offset: usize,
    ) -> Result<(), &str> {
        let mut len = content.len() as isize;
        let bs = self.sb.block_size as usize;
        let mut content = content;
        if offset >= (self.sb.block_size * 12) as usize {
            if offset >= (self.sb.block_size * (12 + self.sb.block_size / 4)) as usize {
                // write double indirect
                self.write_to_double_indirect_block(
                    node.dob_inblock,
                    content,
                    offset - bs * 12 - bs * bs / 4,
                )
                .unwrap();
                return Ok(());
            }
            // write indirect
            let num = self.write_to_indirect_block(node.sin_inblock, content, offset - bs * 12)?;
            // .expect("attemted to write to block 0");
            if num != len as usize {
                self.write_to_double_indirect_block(node.dob_inblock, &content[num..], 0)?;
            }
            return Ok(());
        }
        let mut block_num = offset / bs;

        let start = offset % bs;
        let batch = if (len as usize) < bs - start {
            len as usize
        } else {
            bs - start
        };
        self.get_data_block_mut(node.direct_blocks[block_num])[start..start + batch]
            .copy_from_slice(&content[..batch]);
        content = &content[batch..];
        len -= batch as isize;
        block_num += 1;
        while len > 0 {
            if block_num < 12 {
                let batch = if (len as usize) < bs {
                    len as usize
                } else {
                    bs
                };
                self.get_data_block_mut(node.direct_blocks[block_num])[..batch]
                    .copy_from_slice(&content[..batch]);
                content = &content[batch..];
                len -= batch as isize;
                block_num += 1;
            } else {
                let num = self.write_to_indirect_block(node.sin_inblock, content, 0)?;
                // .expect("attemted to write to block 0");
                if num != len as usize {
                    self.write_to_double_indirect_block(
                        node.dob_inblock,
                        &content[len as usize - num..],
                        0,
                    )?;
                }
                panic!("this happened");
            }
        }

        Ok(())
    }

    pub fn write_file(&mut self, path: &CStr, content: &[u8], offset: usize) -> i32 {
        // println!("write {content:?} to offset {offset}");
        if let Some((node, id)) = self.find_file_mut(path.to_str().unwrap()) {
            let len = content.len();
            let size = self.calculate_size(&node);
            println!("len {}", content.len());
            if size < len + offset {
                if self.truncate(path, len + offset).is_err() {
                    println!("TRUNCATE FAILED");
                    return -1;
                }
                println!("TRUNCATED");
            }
            let mut node = self.get_inode_by_id(id);
            if node.size < (len + offset) as u32 {
                node.size = (len + offset) as u32;
                self.save_inode(id, node);
            }
            // println!("{:?}", node.direct_blocks);
            if self.write_file_data(&node, content, offset).is_err() {
                return 0;
            }
            return content.len() as i32;
        } else {
            println!("NOT FOUND FILE FOR WRITE");
            return 0;
        }
    }

    pub fn read_file(&self, path: &CStr) -> Result<Vec<u8>, &str> {
        if let Some(node) = self.find_file(path.to_str().unwrap()) {
            // println!(
            //     "reading from node {:#?} block {}",
            //     node, node.direct_blocks[0]
            // );
            return self.get_file_data(&node);
        }
        Ok(vec![])
    }

    fn create_file_inter(
        &mut self,
        path: &CStr,
        content: &[u8],
        type_perm: u16,
    ) -> Result<(), &str> {
        // if !(path.count_bytes() > 0 && &path.to_str().unwrap()[0..1] == "/") {
        //     return Err("invalid path");
        // }
        // let path = &path[1..];
        let path_str = path.to_str().unwrap();
        let mut node;
        let node_id;
        let filename;

        if let Some(end) = path_str.rfind('/') {
            if end == 0 {
                node = self.get_inode_by_id(1);
                node_id = 1;
                filename = &path[1..];
            } else {
                (node, node_id) = self
                    .find_file_mut(&path_str[0..end])
                    .expect("file not found");
                filename = &path[end + 1..];
            }
        } else {
            return Err("invalid path");
        }

        if self
            .search_directory(&mut node, filename.to_str().unwrap())
            .is_some()
        {
            return Err("file already exists");
        }
        let name = filename.to_bytes();

        let inode_num = self.inode_bitmap.get_first_free().ok_or("OUT OF MEMORY")?;

        //create dentry
        // if node.direct_blocks[0] == 0 {
        //     node.direct_blocks[0] = self.blocks_bitmap.get_first_free() as u32;
        //     println!("occupy block");
        // }

        self.create_dentry(&node, node_id, inode_num as u32, name);

        //create inode
        if content.len() > 0 || type_perm & 0x4000 != 0 {
            let block_num = self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")?;
            self.create_inode(inode_num, block_num, content.len() as u32, type_perm);

            //create data block
            self.get_data_block_mut(block_num as u32)[0..content.len()].copy_from_slice(content);
            if type_perm & 0x4000 != 0 {
                println!("{:?} created block {}", path, block_num);
                let mut data = [0u8; 22];
                data[..4].copy_from_slice(&(inode_num as u32).to_le_bytes());
                data[4..8].copy_from_slice(&1u32.to_le_bytes());
                data[8..9].copy_from_slice(".".as_bytes());

                let parent_id = self
                    .search_directory_get_id(&node, ".")
                    .expect("parent does not have \".\"");
                data[12..16].copy_from_slice(&(parent_id as u32).to_le_bytes());
                data[16..20].copy_from_slice(&2u32.to_le_bytes());
                data[20..22].copy_from_slice("..".as_bytes());
                self.get_data_block_mut(block_num as u32)[0..data.len()].copy_from_slice(&data);
            }
        } else {
            self.create_inode(inode_num, 0, content.len() as u32, type_perm);
        }

        Ok(())
    }

    fn create_dentry(&mut self, node: &inode_t, id: u32, inode_num: u32, name: &[u8]) {
        let data = self.get_dir_data(&node);
        let mut node = *node;
        let offset = if let Some(offset) = FileSystem::find_space_for_dentry(&data, name.len() + 8)
        {
            offset
        } else {
            let size = self.calculate_size(&node);
            let mut v = vec![];
            v.extend_from_slice(name);
            println!("ADD NEW BLOCK TO DIR WHEN {:?}", String::from_utf8(v));
            self.truncate_inter(node, id, (size + 1) as isize).unwrap();
            node = self.get_inode_by_id(id);
            size
        };

        let name_len = name.len();
        let mut dentry = Vec::with_capacity(8 + name_len);

        dentry.extend_from_slice(&(inode_num).to_le_bytes());
        dentry.extend_from_slice(&(name_len as u32).to_le_bytes());
        dentry.extend_from_slice(name);

        self.write_file_data(&node, &dentry, offset).unwrap();
    }

    fn find_space_for_dentry(data: &[u8], required_size: usize) -> Option<usize> {
        let mut start = 0;
        let mut found = 0;
        let mut i = 0;
        while i + 4 < data.len() {
            if u32::from_le_bytes(data[i..i + 4].try_into().unwrap()) == 0 {
                if found > 0 {
                    found += 4;
                    if found >= required_size {
                        return Some(start);
                    }
                } else {
                    start = i;
                    found = 4;
                }
            } else {
                found = 0;
            }
            i += 4;
        }

        None
    }

    pub fn get_files_in_dir(&self, path: &CStr) -> Vec<String> {
        let mut files = vec![];
        if let Some(node) = self.find_file(path.to_str().unwrap()) {
            if node.is_directory() {
                let d = self.get_dir_data(&node);
                let mut data = &d[..];
                while let Some(dentry) = Dentry::from(data) {
                    files.push(String::from(dentry.name));
                    data = &data[dentry.size..];
                }
            }
        }
        return files;
    }

    fn read_indirect_block(
        &self,
        data: &mut Vec<u8>,
        block_num: u32,
        mut size: usize,
    ) -> Result<usize, &'static str> {
        let mut indirect = self.get_data_block(block_num);
        while indirect.len() > 0 {
            let b = u32::from_le_bytes(indirect[..4].try_into().unwrap());
            if b != 0 {
                if size >= self.sb.block_size as usize {
                    data.extend_from_slice(self.get_data_block(b));
                    size -= self.sb.block_size as usize;
                } else {
                    if size == 0 {
                        return Err("file has more blocks than it should");
                    }
                    data.extend_from_slice(&self.get_data_block(b)[..size]);
                    size = 0;
                }
            } else {
                break;
            }
            indirect = &indirect[4..];
        }
        Ok(size)
    }

    fn read_double_indirect_block(
        &self,
        data: &mut Vec<u8>,
        block_num: u32,
        mut size: usize,
    ) -> Result<usize, &'static str> {
        let mut indirect = self.get_data_block(block_num);
        while indirect.len() > 0 {
            let b = u32::from_le_bytes(indirect[..4].try_into().unwrap());
            if b != 0 {
                if size >= self.sb.block_size as usize {
                    size = self.read_indirect_block(data, b, size)?;
                } else {
                    if size == 0 {
                        return Err("file has more blocks than it should");
                    }
                    size = self.read_indirect_block(data, b, size)?;
                }
            } else {
                break;
            }
            indirect = &indirect[4..];
        }
        Ok(size)
    }
    fn get_file_data(&self, node: &inode_t) -> Result<Vec<u8>, &'static str> {
        let mut data = vec![];
        let mut size = node.size as usize;
        let mut blocks = 0;
        for i in node.direct_blocks {
            if i != 0 {
                if size >= self.sb.block_size as usize {
                    data.extend_from_slice(self.get_data_block(i));
                    size -= self.sb.block_size as usize;
                    blocks += 1;
                } else {
                    if size == 0 {
                        println!("{:?}", node);
                        return Err("file has more blocks than it should");
                    }
                    data.extend_from_slice(&self.get_data_block(i)[..size]);
                    blocks += 1;
                    size = 0;
                }
            }
        }
        if node.sin_inblock != 0 {
            size = self.read_indirect_block(&mut data, node.sin_inblock, size)?;
        }
        if node.dob_inblock != 0 {
            self.read_double_indirect_block(&mut data, node.dob_inblock, size)?;
        }
        println!("read from {blocks} BLOCKS");
        Ok(data)
    }

    fn get_dir_data(&self, node: &inode_t) -> Vec<u8> {
        let mut data = vec![];
        for i in node.direct_blocks {
            if i != 0 {
                data.extend_from_slice(self.get_data_block(i));
            }
        }
        if node.sin_inblock != 0 {
            let mut indirect = self.get_data_block(node.sin_inblock);
            while indirect.len() > 0 {
                let b = u32::from_le_bytes(indirect[..4].try_into().unwrap());
                if b != 0 {
                    data.extend_from_slice(self.get_data_block(b));
                } else {
                    break;
                }
                indirect = &indirect[4..];
            }
        }
        data
    }

    /*pub fn dummy_data(&mut self) {
        println!(
            "create foo {:?}",
            self.create_file(
                c"/foo",
                &['L' as u8, 'O' as u8, 'L' as u8, 0],
                0x8000 | 0o666
            )
        );
        println!(
            "create boo {:?}",
            self.create_file(
                c"/boo",
                &['M' as u8, 'O' as u8, 'L' as u8, 0],
                0x8000 | 0o666
            )
        );
        println!("{:?}", self.create_directory(c"/XD"));
        println!(
            "{:?}",
            self.create_file(c"/XD/xd", &[1u8, 2, 3, 4, 5], 0x8000 | 0o666)
        );
        // self.create_file(
        //     c"goo",
        //     &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        // );
        println!("READS");
        println!(
            "foo: {:?}",
            CStr::from_bytes_until_nul(&self.read_file(c"/foo").unwrap())
        );
        println!("foo: {:?}", self.read_file(c"/foo").unwrap());
        println!("boo: {:?}", self.read_file(c"/boo").unwrap());
        println!("XD: {:?}", self.read_file(c"/XD").unwrap());
        println!("xd: {:?}", self.read_file(c"/XD/xd").unwrap());
        self.write_file(c"/XD/xd", "LOL".as_bytes(), 0);
        self.write_file(c"/XD/xd", "XD".as_bytes(), 3);
        self.write_file(c"/XD/xd", "FOO".as_bytes(), 0);
        println!("xd: {:?}", self.read_file(c"/XD/xd").unwrap());
        println!("{:?}", self.create_file(c"/XD/xd", &[0u8], 0x8000 | 0o666));
        println!("{:?}", self.create_directory(c"/XD/LUL"));
        println!(
            "{:?}",
            self.create_file(c"/XD/LUL/cos tam", &[5u8, 5, 5], 0x8000 | 0o666),
        );
        println!("xd: {:?}", self.read_file(c"/XD/LUL/cos tam").unwrap());
        println!("DELETE");
        println!("{:?}", self.unlink_file("/boo"));
        println!("XD: {:?}", self.read_file(c"/XD").unwrap());
        println!("xd: {:?}", self.read_file(c"/XD/LUL/cos tam").unwrap());
        println!("del XD {:?}", self.unlink_dir(c"/XD"));
        println!(
            "{:?}",
            self.create_file(
                c"/asdfghjkl",
                &['M' as u8, 'O' as u8, 'L' as u8, 0],
                0x8000 | 0o666
            )
        );
        println!(
            "{:?}",
            self.create_file(c"/g", &['M' as u8, 'O' as u8, 'L' as u8, 0], 0x8000 | 0o666)
        );
        self.unlink_file("/g").expect("failed to delete");
        self.unlink_file("/XD/LUL/cos tam")
            .expect("failed to delete");
        println!("delete LUL {:?}", self.unlink_dir(c"/XD/LUL"));
        println!(
            "{:?}",
            self.create_file(
                c"/qwertyui",
                &['M' as u8, 'O' as u8, 'L' as u8, 0],
                0x8000 | 0o666
            )
        );
    }*/

    fn clear_dentry(&mut self, node: &inode_t, filename: &str) {
        let mut i = 0usize;
        let mut data = self.get_dir_data(node);

        // println!("searching filename {}", filename);
        while let Some(dentry) = DentryMut::from(&mut data[i..]) {
            println!("{i}");
            if dentry.get_name() == filename {
                // println!("{:?} {} {} {}", dentry.get_name(), filename, i, dentry.size);
                self.write_file_data(node, &vec![0; dentry.size], i)
                    .unwrap();
                // println!("{:?}", self.get_dir_data(node));
                return;
            }
            i += dentry.size;
        }
        panic!("tried to delete inexisting entry");
    }

    fn search_directory_get_id(&self, node: &inode_t, filename: &str) -> Option<inode_p> {
        let mut i = 0usize;
        let data = self.get_dir_data(node);

        //println!("searching filename {}", filename);
        while let Some(dentry) = Dentry::from(&data[i..]) {
            if dentry.name == filename {
                //println!("inode num {}", dentry.inode_num);
                return Some(dentry.inode_num);
            } else {
                // println!("{:?} {:?}", dentry.name.as_bytes(), filename.as_bytes());
                // println!("{:?} {:?}", dentry.name, filename);
            }
            i += dentry.size;
            // println!("{i}");
        }
        None
    }

    fn search_directory(&self, node: &inode_t, filename: &str) -> Option<inode_t> {
        if let Some(id) = self.search_directory_get_id(node, filename) {
            return Some(self.get_inode_by_id(id));
        }
        None
    }

    fn calculate_size(&self, node: &inode_t) -> usize {
        let mut size = 0;
        for i in node.direct_blocks {
            if i != 0 {
                size += self.sb.block_size as usize;
            } else {
                break;
            }
        }

        if node.sin_inblock != 0 {
            let mut indirect = self.get_data_block(node.sin_inblock);
            while indirect.len() > 0 {
                let b = u32::from_le_bytes(indirect[..4].try_into().unwrap());
                if b != 0 {
                    size += self.sb.block_size as usize;
                } else {
                    break;
                }
                indirect = &indirect[4..];
            }
        }
        size
    }

    fn truncate_indirect_block(
        &mut self,
        block_num: u32,
        mut size: isize,
    ) -> Result<isize, &'static str> {
        let mut i = 0;
        // println!(
        //     "indblock {} {:?}",
        //     block_num,
        //     self.get_data_block(block_num)
        // );
        while self.get_data_block(block_num).len() - i > 0 {
            let b =
                u32::from_le_bytes(self.get_data_block(block_num)[i..i + 4].try_into().unwrap());
            if size > 0 {
                if b == 0 {
                    let free = self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")? as u32;
                    self.get_data_block_mut(free).zero();
                    self.get_data_block_mut(block_num)[i..i + 4]
                        .copy_from_slice(&free.to_le_bytes());
                }
                size -= self.sb.block_size as isize;
            } else {
                if b != 0 {
                    self.blocks_bitmap.free(b as usize);
                    self.get_data_block_mut(block_num)[i..i + 4]
                        .copy_from_slice(&0u32.to_le_bytes());
                }
            }
            i += 4;
        }
        Ok(size)
    }

    fn truncate_doubly_indirect_block(
        &mut self,
        block_num: u32,
        mut size: isize,
    ) -> Result<isize, &'static str> {
        let mut i = 0;
        // println!(
        //     "indblock {} {:?}",
        //     block_num,
        //     self.get_data_block(block_num)
        // );
        while self.get_data_block(block_num).len() - i > 0 {
            let mut b =
                u32::from_le_bytes(self.get_data_block(block_num)[i..i + 4].try_into().unwrap());
            if size > 0 {
                if b == 0 {
                    b = self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")? as u32;
                    self.get_data_block_mut(b).zero();
                }
                // println!("before indirect {size}");
                size = self.truncate_indirect_block(b, size)?;
                // println!("after indirect {size}");
                self.get_data_block_mut(block_num)[i..i + 4].copy_from_slice(&b.to_le_bytes());
            } else {
                if b != 0 {
                    self.truncate_indirect_block(b, size)?;
                    self.blocks_bitmap.free(b as usize);
                    self.get_data_block_mut(block_num)[i..i + 4]
                        .copy_from_slice(&0u32.to_le_bytes());
                }
            }
            i += 4;
        }
        Ok(size)
    }

    fn truncate_inter(&mut self, mut node: inode_t, id: u32, mut size: isize) -> Result<(), &str> {
        node.size = size as u32;
        for i in node.direct_blocks.iter_mut() {
            // println!("{size} {}", *i);
            if size > 0 {
                if *i == 0 {
                    *i = self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")? as u32;
                    self.get_data_block_mut(*i).zero();
                    // println!("{}", *i);
                }
                size -= self.sb.block_size as isize;
            } else {
                if *i != 0 {
                    self.blocks_bitmap.free(*i as usize);
                    *i = 0;
                }
            }
        }
        if size > 0 {
            // create indirect block
            if node.sin_inblock == 0 {
                node.sin_inblock =
                    self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")? as u32;
                self.get_data_block_mut(node.sin_inblock).zero();
            }
            size = self.truncate_indirect_block(node.sin_inblock, size)?;
            // println!("{:?}", self.get_data_block(node.sin_inblock));
        } else {
            // delete indirect block
            if node.sin_inblock != 0 {
                self.truncate_indirect_block(node.sin_inblock, size)?;
                self.blocks_bitmap.free(node.sin_inblock as usize);
                node.sin_inblock = 0;
            }
        }
        if size > 0 {
            // create double indirect
            println!("size: {size}");
            if node.dob_inblock == 0 {
                node.dob_inblock =
                    self.blocks_bitmap.get_first_free().ok_or("OUT OF MEMORY")? as u32;
                self.get_data_block_mut(node.dob_inblock).zero();
            }
            size = self
                .truncate_doubly_indirect_block(node.dob_inblock, size)
                .unwrap();
        } else {
            // delete doubly indirect
            self.truncate_doubly_indirect_block(node.dob_inblock, size)
                .unwrap();
            self.blocks_bitmap.free(node.dob_inblock as usize);
            node.dob_inblock = 0;
        }
        println!("size: {size}");
        if size > 0 {
            println!("triple size: {size}");
            // return Err("triply indirect block");
            panic!("triply indirect block");
        }
        println!("{node:?}");
        self.save_inode(id, node);
        return Ok(());
    }

    pub fn truncate(&mut self, path: &CStr, size: usize) -> Result<(), &str> {
        let path = path.to_str().expect("path should be UTF-8");
        let size = size as isize;
        if let Some((node, id)) = self.find_file_mut(path) {
            return self.truncate_inter(node, id, size);
        }

        Err("failed")
    }

    pub fn chmod(&mut self, path: &CStr, mode: u32) -> Result<(), &str> {
        if let Some((mut node, id)) =
            self.find_file_mut(path.to_str().expect("path should be UTF-8"))
        {
            node.type_perm = (mode | node.type_perm as u32 & 0xF000) as u16;
            self.save_inode(id, node);
            return Ok(());
        }
        Err("file not found")
    }

    fn get_inode_by_id(&self, id: inode_p) -> inode_t {
        if id == 0 {
            panic!("invalid inode id");
        }
        let start = (128 * id) as usize;
        let data: [u8; 128] = self.inodes[start..start + 128]
            .try_into()
            .expect(&format!("failed to load inode {}", id));
        zerocopy::transmute!(data)
    }

    // pub fn get_inode(&mut self, path: &CStr) {
    //     let d: [u8; 128] = self.inodes[0..128].try_into().unwrap();
    //     let root: inode_t = zerocopy::transmute!(d);
    //     self.search_directory(&root, path);
    // }

    pub fn create_inode(&mut self, id: usize, first_block: usize, size: u32, type_perm: u16) {
        // let i = self.inode_bitmap.get_first_free();
        let mut blocks = [0u32; 12];
        blocks[0] = first_block as u32; //self.blocks_bitmap.get_first_free() as u32;
                                        // println!("block {}", blocks[0]);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went back")
            .as_secs();
        //let data: [u8; 128] = .try_into().unwrap();
        let node = inode_t {
            type_perm,
            uid: 1000,
            gid: 1000,
            pad1: 0,
            size,
            pad2: 0,
            access_time: time,
            mod_time: time,
            creat_time: time,
            hard_links: 2,
            direct_blocks: blocks,
            sin_inblock: 0,
            dob_inblock: 0,
            tri_inblock: 0,
            unused: [0i8; 24],
        };
        let data: [u8; 128] = zerocopy::transmute!(node);
        self.inodes[id * 128..(id + 1) * 128].copy_from_slice(&data);
    }

    fn save_inode(&mut self, id: inode_p, node: inode_t) {
        let id = id as usize;
        let data: [u8; 128] = zerocopy::transmute!(node);
        self.inodes[id * 128..(id + 1) * 128].copy_from_slice(&data);
    }

    fn get_data_block_mut(&mut self, id: block_p) -> &mut [u8] {
        let offset = (id * self.sb.block_size) as usize;
        return &mut self.data_blocks[offset..offset + self.sb.block_size as usize];
    }

    fn get_data_block(&self, id: block_p) -> &[u8] {
        let offset = (id * self.sb.block_size) as usize;
        return &self.data_blocks[offset..offset + self.sb.block_size as usize];
    }
}

#[derive(Debug)]
struct Bitmap<'a> {
    data: &'a mut [u8],
    size: usize,
}

impl<'a> Bitmap<'a> {
    pub fn new(data: &'a mut [u8], size: usize) -> Self {
        if data.len() * 8 < size {
            panic!("buffer to small to create bitmap");
        }
        Bitmap { data, size }
    }

    pub fn take(&mut self, id: usize) {
        self.data[id / 8] |= 1 << id % 8;
        // println!("{:?}", self.data);
    }

    pub fn free(&mut self, id: usize) {
        self.data[id / 8] &= !(1 << id % 8);
    }

    pub fn get_first_free(&mut self) -> Option<usize> {
        let mut i = 1;
        while i < self.size {
            if self.data[i / 8] & 1 << i % 8 == 0 {
                self.data[i / 8] |= 1 << i % 8;
                return Some(i);
            }
            i += 1;
        }
        return None;
    }
}

#[derive(Debug)]
struct Dentry<'a> {
    inode_num: inode_p,
    name: &'a str,
    size: usize,
}

impl<'a> Dentry<'a> {
    fn from(data: &'a [u8]) -> Option<Self> {
        let mut data = &data[..];
        let mut i = 0;
        // println!("foo {i} {:?}", &data[0..4]);
        while data.len() >= 8 && u32::from_le_bytes(data[0..4].try_into().unwrap()) == 0 {
            // println!("ups");
            i += 4;
            data = &data[4..];
        }
        if data.len() < 8 {
            // println!("too small");
            return None;
            //"dentry too small"
        }
        let inode_num = inode_p::from_le_bytes(data[0..4].try_into().unwrap());
        if inode_num == 0 {
            println!("inode 0");
            return None;
        }
        let size = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if data.len() < (8 + size) as usize {
            println!("size wrong");
            println!("{} {}", data.len(), 8 + size);
            return None;
            //"dir name size incorrect"
        }
        Some(Self {
            inode_num,
            name: std::str::from_utf8(&data[8..8 + size as usize]).expect(&format!(
                "bad file name {:?} size = {}",
                &data[8..8 + size as usize],
                size
            )),
            size: match size % 4 {
                0 => size as usize + 8 + i,
                1 => size as usize + 8 + i + 3,
                2 => size as usize + 8 + i + 2,
                3 => size as usize + 8 + i + 1,
                _ => unreachable!("modulo lol"),
            },
        })
    }
}

struct DentryMut<'a> {
    size: usize,
    data: &'a mut [u8],
}

impl<'a> DentryMut<'a> {
    fn from(data: &'a mut [u8]) -> Option<Self> {
        let data = &mut data[..];
        let mut i = 0;
        while data.len() - i >= 8 && u32::from_le_bytes(data[i..i + 4].try_into().unwrap()) == 0 {
            // data = &mut data[4..];
            i += 4;
        }
        if data.len() < 8 {
            println!("too small");
            return None;
            //"dentry too small"
        }
        let inode_num = inode_p::from_le_bytes(data[i..i + 4].try_into().unwrap());
        if inode_num == 0 {
            println!("inode 0");
            return None;
        }
        let size = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap());
        if data.len() < (8 + size) as usize {
            println!("size wrong");
            return None;
            //"dir name size incorrect"
        }
        Some(Self {
            // size: size as usize + 8,
            size: match size % 4 {
                0 => size as usize + 8 + i,
                1 => size as usize + 8 + i + 3,
                2 => size as usize + 8 + i + 2,
                3 => size as usize + 8 + i + 1,
                _ => unreachable!("modulo lol"),
            },
            data: &mut data[i..i + 8 + size as usize],
        })
    }

    fn get_name(&self) -> &str {
        std::str::from_utf8(&self.data[8..])
            .expect(&format!("bad file name {:?}", &self.data[8..],))
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Copy, Clone, zerocopy_derive::FromBytes, zerocopy_derive::IntoBytes)]
pub struct superblock_t {
    pub header: [::std::os::raw::c_char; 8usize],
    pub inodes_num: ::std::os::raw::c_uint,
    pub blocks_num: ::std::os::raw::c_uint,
    pub block_size: ::std::os::raw::c_uint,
    pub free_blocks: ::std::os::raw::c_uint,
    pub free_inodes: ::std::os::raw::c_uint,
}

#[allow(non_camel_case_types)]
pub type block_p = ::std::os::raw::c_uint;
#[allow(non_camel_case_types)]
pub type inode_p = ::std::os::raw::c_uint;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Copy, Clone, zerocopy_derive::FromBytes, zerocopy_derive::IntoBytes)]
pub struct inode_t {
    pub type_perm: ::std::os::raw::c_ushort,
    pub uid: ::std::os::raw::c_ushort,
    pub gid: ::std::os::raw::c_ushort,
    pub pad1: ::std::os::raw::c_ushort,
    pub size: ::std::os::raw::c_uint,
    pub pad2: ::std::os::raw::c_uint,
    pub access_time: ::std::os::raw::c_ulonglong,
    pub mod_time: ::std::os::raw::c_ulonglong,
    pub creat_time: ::std::os::raw::c_ulonglong,
    pub hard_links: ::std::os::raw::c_uint,
    pub direct_blocks: [block_p; 12usize],
    pub sin_inblock: block_p,
    pub dob_inblock: block_p,
    pub tri_inblock: block_p,
    pub unused: [::std::os::raw::c_char; 24usize],
}

impl inode_t {
    pub fn is_directory(&self) -> bool {
        self.type_perm & 0x4000 != 0
    }
}
