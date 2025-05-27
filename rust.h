#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct FileSystem FileSystem;

typedef unsigned int block_p;

typedef struct inode_t {
  unsigned short type_perm;
  unsigned short uid;
  unsigned short gid;
  unsigned short pad1;
  unsigned int size;
  unsigned int pad2;
  unsigned long long access_time;
  unsigned long long mod_time;
  unsigned long long creat_time;
  unsigned int hard_links;
  block_p direct_blocks[12];
  block_p sin_inblock;
  block_p dob_inblock;
  block_p tri_inblock;
  char unused[24];
} inode_t;

typedef struct superblock_t {
  char header[8];
  unsigned int inodes_num;
  unsigned int blocks_num;
  unsigned int block_size;
  unsigned int free_blocks;
  unsigned int free_inodes;
} superblock_t;

typedef unsigned int inode_p;

int32_t rs_getattr(struct FileSystem *fs, const char *filename, struct inode_t *inode_buf);

int32_t rs_open(struct FileSystem *fs, const char *filename);

int32_t rs_read(struct FileSystem *fs, const char *filename, int8_t *buf, uintptr_t size);

int32_t rs_readdir(struct FileSystem *fs, const char *filename, void *buf, fuse_fill_dir_t filler);

int32_t rs_create(struct FileSystem *fs, const char *filename);

int32_t rs_write(struct FileSystem *fs, const char *filename, const char *content, uintptr_t size);

int32_t rs_mkdir(struct FileSystem *fs, const char *filename);

int32_t rs_unlink(struct FileSystem *fs, const char *filename);

int32_t rs_rmdir(struct FileSystem *fs, const char *filename);

int32_t rs_rename(struct FileSystem *fs, const char *from, const char *to);

struct FileSystem *rs_init(void);

extern void *get_block(struct superblock_t *sb, block_p id);

extern struct inode_t *get_inode_by_id(struct superblock_t *sb, inode_p id);

extern void take_inode(struct superblock_t *sb, inode_p id);

extern block_p get_free_data_block(struct superblock_t *sb);

extern void test(struct superblock_t *sb);

extern inode_p find_free_inode(struct superblock_t *sb);

extern struct inode_t *find_inode_by_path(struct superblock_t *sb, const char *path);

extern struct inode_t *search_dir(struct inode_t *cur, const char *path);
