#include <stdio.h>
#include <string.h>
#include <sys.h>
#include <file.h>

#define TICKS_PER_SECOND 60
#define DEFAULT_ITERATIONS 10000
#define DEFAULT_RW_SIZE 512
#define MAX_RW_SIZE 2048

static int parse_uint(char *s)
{
	int value = 0;
	if (s == 0 || *s == 0)
		return -1;

	while (*s)
	{
		if (*s < '0' || *s > '9')
			return -1;
		value = value * 10 + (*s - '0');
		s++;
	}
	return value;
}

static int total_ticks(struct tms *t)
{
	return t->utime + t->stime;
}

static void print_usage()
{
	printf("Usage:\n");
	printf("  syscall_bench openclose <path> [iterations]\n");
	printf("  syscall_bench stat <path> [iterations]\n");
	printf("  syscall_bench seek <path> [iterations]\n");
	printf("  syscall_bench read <path> [iterations] [size]\n");
	printf("  syscall_bench write <path> [iterations] [size]\n");
}

// syscall_bench xxx ls/tmp
int main1(int argc, char *argv[])
{
	struct tms pre, post;
	struct st_inode st;
	char rwbuf[MAX_RW_SIZE];
	char *op;
	char *path;
	int iterations = DEFAULT_ITERATIONS;
	int rw_size = DEFAULT_RW_SIZE;
	int i;
	int fd;
	int ret;
	int ticks;
	int total_us;
	int avg_us;

	if (argc < 3)
	{
		print_usage();
		return -1;
	}

	op = argv[1];
	path = argv[2];

	if (argc >= 4)
	{
		iterations = parse_uint(argv[3]);
		if (iterations <= 0)
			iterations = DEFAULT_ITERATIONS;
	}

	if (argc >= 5)
	{
		rw_size = parse_uint(argv[4]);
		if (rw_size <= 0 || rw_size > MAX_RW_SIZE)
			rw_size = DEFAULT_RW_SIZE;
	}

	for (i = 0; i < rw_size; i++)
		rwbuf[i] = 'A' + (i % 26);

	memset((void *)&pre, 0, sizeof(pre));
	memset((void *)&post, 0, sizeof(post));
	times(&pre);

	if (!strcmp(op, "openclose"))
	{
		for (i = 0; i < iterations; i++)
		{
			fd = open(path, 0x1);
			if (fd < 0)
			{
				printf("open failed at iteration %d\n", i);
				return -1;
			}
			close(fd);
		}
	}
	else if (!strcmp(op, "stat"))
	{
		for (i = 0; i < iterations; i++)
		{
			ret = stat(path, (unsigned long)&st);
			if (ret < 0)
			{
				printf("stat failed at iteration %d\n", i);
				return -1;
			}
		}
	}
	else if (!strcmp(op, "seek"))
	{
		fd = open(path, 0x1);
		if (fd < 0)
		{
			printf("open failed\n");
			return -1;
		}

		for (i = 0; i < iterations; i++)
		{
			ret = seek(fd, 0, 0);
			if (ret < 0)
			{
				printf("seek failed at iteration %d\n", i);
				close(fd);
				return -1;
			}
		}
		close(fd);
	}
	else if (!strcmp(op, "read"))
	{
		fd = open(path, 0x1);
		if (fd < 0)
		{
			printf("open failed\n");
			return -1;
		}

		for (i = 0; i < iterations; i++)
		{
			seek(fd, 0, 0);
			ret = read(fd, rwbuf, rw_size);
			if (ret < 0)
			{
				printf("read failed at iteration %d\n", i);
				close(fd);
				return -1;
			}
		}
		close(fd);
	}
	else if (!strcmp(op, "write"))
	{
		fd = creat(path, 0x1FF);
		if (fd < 0)
		{
			printf("creat failed\n");
			return -1;
		}

        iterations = 5000;
		for (i = 0; i < iterations; i++)
		{
			seek(fd, 0, 0);
			ret = write(fd, rwbuf, rw_size);
			if (ret != rw_size)
			{
				printf("write failed at iteration %d\n", i);
				close(fd);
				return -1;
			}
		}
		//syncFileSystem();
		close(fd);
	}
	else
	{
		print_usage();
		return -1;
	}

	times(&post);

	ticks = total_ticks(&post) - total_ticks(&pre);
	if (ticks < 0)
		ticks = 0;

	total_us = ticks * 1000000 / TICKS_PER_SECOND;
	avg_us = 0;
	if (iterations > 0)
		avg_us = total_us / iterations;

	printf("Syscall benchmark result:\n");
	printf("operation      : %s\n", op);
	printf("path           : %s\n", path);
	printf("iterations     : %d\n", iterations);
	printf("rw_size        : %d\n", rw_size);
	printf("total_us_est   : %d\n", total_us);
	printf("avg_us_est     : %d\n", avg_us);

	return 0;
}
