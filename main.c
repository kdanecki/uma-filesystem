#include "stdio.h"
#include <asm-generic/errno-base.h>
#include <stdlib.h>
#include <string.h>
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
        stbuf->st_mode = node.type_perm;
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
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_read(fs, path, buf, size, offset);
    if (res == -1)
        return -EFAULT;
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
    if (rs_create(fs, path, mode) == 0)
        return 0;
    
    return -EPERM;
}

int c_write(const char* path, const char* buf, size_t size, off_t off, struct fuse_file_info* fi)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int ret = rs_write(fs, path, buf, size, off);
    if (ret == -1)
        return -EFBIG;
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

int c_chmod(const char* path, mode_t mode)
{
    struct FileSystem *fs = (struct FileSystem*) fuse_get_context()->private_data;
    int res = rs_chmod(fs, path, mode);
    return res ? -ENOENT : 0;
}

int c_release(const char * path, struct fuse_file_info* fi)
{
    return 0;
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
    .chmod = c_chmod,
    .release = c_release,
};

void print_version()
{
    printf("oxidizedFS 0.1\n");
}

void print_usage()
{
    printf("Syntax: oxidizedFS [OPTIONS] [disk image name] [COMMAND]\n\n"
           "Options:\n"
           "  -v\t\tshow version and quit\n"
           "  -h\t\tshow this help message and quit\n"
           "\n"
           "Commands:\n"
           "  format <block size> <block num> <inode num>\tcreates image with given name\n" 
           "  mount <fuse args>\t\t\t\tmounts the filesystem\n");
    
}

int format(int argc, char* argv[])
{
    if (argc < 6)
    {
        print_usage();
        return 1;
    }
    FileSystem* fs = rs_init_and_format(argv[1], atoll(argv[3]), atoll(argv[4]), atoll(argv[5]));
    return 0;
}

int my_mount(int argc, char** argv)
{
    FileSystem* fs = rs_init(argv[1]);
    argv[2] = argv[0];
    char** lol = &argv[2];
    return fuse_main(argc-2, lol, &my_oper, fs);
}

int main(int argc, char *argv[])
{
    // handle options
    if (argc < 2)
    {
        print_usage();
        return 1;
    }
    if (strcmp(argv[1], "-h") == 0)
    {
        print_usage();
        return 0;
    }
    if (strcmp(argv[1], "-v") == 0)
    {
        print_version();
        return 0;
    }

    // handle commands
    if (argc < 3)
    {
        print_usage();
        return 1;
    }
    if (strcmp(argv[2], "format") == 0)
    {
        return format(argc, argv);
    }
    if (strcmp(argv[2], "mount") == 0)
    {
        return my_mount(argc, argv);
    }
    print_usage();
    return 1;
    
    // printf("argc %d\n", argc);
    // for (int i =0; i < argc ; i++)
    // {
    //     printf("argv[%d] %s\n", i, argv[i]);
    // }
}

