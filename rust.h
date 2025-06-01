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

int32_t rs_getattr(struct FileSystem *fs, const char *filename, struct inode_t *inode_buf);

int32_t rs_open(struct FileSystem *fs, const char *filename);

int32_t rs_read(struct FileSystem *fs,
                const char *filename,
                int8_t *buf,
                uintptr_t size,
                uintptr_t offset);

int32_t rs_readdir(struct FileSystem *fs, const char *filename, void *buf, fuse_fill_dir_t filler);

int32_t rs_create(struct FileSystem *fs, const char *filename, uint32_t mode);

int32_t rs_write(struct FileSystem *fs,
                 const char *filename,
                 const char *content,
                 uintptr_t size,
                 uintptr_t offset);

int32_t rs_mkdir(struct FileSystem *fs, const char *filename);

int32_t rs_unlink(struct FileSystem *fs, const char *filename);

int32_t rs_rmdir(struct FileSystem *fs, const char *filename);

int32_t rs_truncate(struct FileSystem *fs, const char *filename, uintptr_t size);

int32_t rs_rename(struct FileSystem *fs, const char *from, const char *to);

int32_t rs_chmod(struct FileSystem *fs, const char *filename, uint32_t mode);

struct FileSystem *rs_init(const char *filename);

struct FileSystem *rs_init_and_format(const char *filename,
                                      uint64_t block_size,
                                      uint64_t block_num,
                                      uint32_t inode_num);
