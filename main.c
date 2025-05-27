#include "stdio.h"
#include <asm-generic/errno-base.h>
#include <sys/types.h>
#define FUSE_USE_VERSION 26
#include <fuse.h>
#include <errno.h>
#include "rust.h"


int c_getattr(const char* path, struct stat* stbuf)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    printf("XD: %s\n", path);
    struct inode_t node;
    int res = rs_getattr(fs, path, &node);
    if (res == 0)
    {
        stbuf->st_mode = node.type_perm == 2 ? S_IFDIR | 0755 : S_IFREG | 0666;
    		stbuf->st_nlink = node.hard_links;
    		stbuf->st_uid = node.uid;
    		stbuf->st_gid = node.gid;
		
    		stbuf->st_atime = node.access_time;
    		stbuf->st_mtime = node.mod_time;
    		stbuf->st_ctime = node.creat_time;
    		stbuf->st_size = node.size;
    		stbuf->st_blocks = 2;
    		return 0;
    }
    return -ENOENT;
}

int c_open(const char* path, struct fuse_file_info* fi)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    return rs_open(fs, path) ? -ENOENT : 0;
}

int c_read(const char* path, char* buf, size_t size, off_t offset, struct fuse_file_info* fi)
{
    if (offset != 0)
        return 0;
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_read(fs, path, buf, size);
    return res ? res : -ENOENT;
}

int c_readdir(const char *path, void *buf, fuse_fill_dir_t filler,
			 off_t offset, struct fuse_file_info *fi)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    if (rs_readdir(fs, path, buf, filler) == 0)
    {
        return 0;
    }
    return -ENOENT;
}

int c_create(const char* path, mode_t mode, struct fuse_file_info* fi)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    if (rs_create(fs, path) == 0)
        return 0;
    
    return -EPERM;
}

int c_write(const char* path, const char* buf, size_t size, off_t off, struct fuse_file_info* fi)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int ret = rs_write(fs, path, buf, size);
    if (ret == -1)
        return -ENOMEM;
    return ret ? ret : -ENODATA;
}

int c_utimens()
{
    return 0;
}

int c_truncate(const char* path, off_t size)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int ret = rs_truncate(fs, path, size);
    return ret ? -ENOENT : 0;
}

int c_chown()
{
    return 0;
}

int c_rename(const char* from, const char* to)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_rename(fs, from, to);
    return res ? -ENOENT : 0;
}

int c_mkdir(const char *path, mode_t mode)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_mkdir(fs, path);
    return res ? -EEXIST : 0;
}

int c_unlink(const char* path)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_unlink(fs, path);
    return res ? -ENOENT : 0;
}

int c_rmdir(const char* path)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_rmdir(fs, path);
    return res ? -ENOENT : 0;
}

static struct fuse_operations my_oper = {
    .getattr = c_getattr,
    .open = c_open,
    .read = c_read,
    .readdir = c_readdir,
    .create = c_create,
    .write = c_write,
    .utimens = c_utimens,
    .truncate = c_truncate,
    .chown = c_chown,
    .mkdir = c_mkdir,
    .unlink = c_unlink,
    .rmdir = c_rmdir,
    .rename = c_rename,
};

int main(int argc, char *argv[])
{
    FileSystem* fs = rs_init();
    char buf[1024];
    printf("%d\n",rs_read(fs, "foo", buf, 1024));
    printf("%s", buf);
    return fuse_main(argc, argv, &my_oper, fs);
}
