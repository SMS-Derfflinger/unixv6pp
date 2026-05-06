#include <stdio.h>
#include <stdlib.h>
#include <sys.h>
#include <file.h>
#include <time.h>

#define TICKS_PER_SECOND 60
#define DEFAULT_BUF_SIZE 512
#define DEFAULT_REPEAT 5
#define MAX_BUF_SIZE 4096

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

static void print_usage()
{
	printf("Usage: io_throughput <src> <dst> [buf_size] [repeat]\n");
}

static int total_ticks(struct tms *t)
{
	return t->utime + t->stime;
}

// io_throughput ls tmp
int main1(int argc, char *argv[])
{
	struct tms pre, post;
	int fdSrc = -1;
	int fdDst = -1;
	int repeat = DEFAULT_REPEAT;
	int i;
	int rbytes;
	int wbytes;
	long total_bytes = 0;
	long ticks;
	long elapsed_us;
	long long bytes_per_sec;
	long kb_per_sec;
	char buf[DEFAULT_BUF_SIZE];

	if (argc < 3)
	{
		print_usage();
		return -1;
	}

	if (argc >= 5)
	{
		repeat = parse_uint(argv[4]);
		if (repeat <= 0)
			repeat = DEFAULT_REPEAT;
	}

	fdSrc = open(argv[1], 0x1);

	fdDst = creat(argv[2], 0x1FF);
	if (fdDst < 0)
	{
		printf("Cannot create target file: %s\n", argv[2]);
		close(fdSrc);
		return -1;
	}

	memset((void *)&pre, 0, sizeof(pre));
	memset((void *)&post, 0, sizeof(post));
	times(&pre);

	for (i = 0; i < repeat; i++)
	{
		seek(fdSrc, 0, 0);
		seek(fdDst, 0, 0);

		while ((rbytes = read(fdSrc, buf, DEFAULT_BUF_SIZE)) > 0)
		{
			wbytes = write(fdDst, buf, rbytes);
			if (wbytes != rbytes)
			{
				printf("write failed in round %d\n", i);
				close(fdSrc);
				close(fdDst);
				return -1;
			}
			total_bytes += wbytes;
		}

		if (rbytes < 0)
		{
			printf("read failed in round %d\n", i);
			close(fdSrc);
			close(fdDst);
			return -1;
		}
	}

	times(&post);

	close(fdSrc);
	close(fdDst);

	ticks = total_ticks(&post) - total_ticks(&pre);
	if (ticks < 0)
		ticks = 0;

	elapsed_us = ticks * 1000000 / TICKS_PER_SECOND;
	if (elapsed_us == 0)
		elapsed_us = 1;

	bytes_per_sec = total_bytes  * TICKS_PER_SECOND / ticks;
	kb_per_sec = ((total_bytes / 1024L) * 1000000L) / elapsed_us;

	printf("I/O throughput benchmark result:\n");
	printf("source         : %s\n", argv[1]);
	printf("target         : %s\n", argv[2]);
	printf("buf_size       : %d\n", DEFAULT_BUF_SIZE);
	printf("repeat         : %d\n", repeat);
	printf("total_bytes    : %d\n", total_bytes);
	printf("elapsed_us_est : %d\n", elapsed_us);
	printf("bytes_per_sec  : %ld\n", bytes_per_sec);
	printf("kb_per_sec     : %ld\n", kb_per_sec);

	return 0;
}
