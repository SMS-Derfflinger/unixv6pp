#include <stdio.h>
#include <string.h>
#include <malloc.h>
#include <sys.h>
#include <time.h>

#define TICKS_PER_SECOND 60
#define DEFAULT_SIZE 64
#define DEFAULT_COUNT 20
#define DEFAULT_ROUNDS 50
#define MAX_COUNT 512

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
	printf("Usage: malloc_bench [size] [count] [rounds]\n");
}

// malloc_bench 
int main1(int argc, char *argv[])
{
	int size = DEFAULT_SIZE;
	int count = DEFAULT_COUNT;
	int rounds = DEFAULT_ROUNDS;
	int i, r;
	int total_ops;
	int ticks;
	int total_us;
	int avg_us;
	struct tms pre, post;
	void *ptrs[MAX_COUNT];

	if (argc >= 2)
	{
		size = parse_uint(argv[1]);
		if (size <= 0)
			size = DEFAULT_SIZE;
	}

	if (argc >= 3)
	{
		count = parse_uint(argv[2]);
		if (count <= 0 || count > MAX_COUNT)
			count = DEFAULT_COUNT;
	}

	if (argc >= 4)
	{
		rounds = parse_uint(argv[3]);
		if (rounds <= 0)
			rounds = DEFAULT_ROUNDS;
	}

	if (count > MAX_COUNT)
	{
		print_usage();
		return -1;
	}

	for (i = 0; i < count; i++)
		ptrs[i] = 0;

	memset((void *)&pre, 0, sizeof(pre));
	memset((void *)&post, 0, sizeof(post));
	times(&pre);

	for (r = 0; r < rounds; r++)
	{
		for (i = 0; i < count; i++)
		{
			ptrs[i] = malloc(size);
			if (ptrs[i] == 0)
			{
				printf("malloc failed: round=%d index=%d size=%d\n", r, i, size);
				return -1;
			}
		}

		for (i = 0; i < count; i++)
		{
			free(ptrs[i]);
			ptrs[i] = 0;
		}
	}

	times(&post);

	total_ops = count * rounds;
	ticks = total_ticks(&post) - total_ticks(&pre);
	if (ticks < 0)
		ticks = 0;

	total_us = ticks * 1000000 / TICKS_PER_SECOND;
	avg_us = 0;
	if (total_ops > 0)
		avg_us = total_us / (total_ops * 2);

	printf("Malloc benchmark result:\n");
	printf("alloc_size     : %d\n", size);
	printf("count_per_round: %d\n", count);
	printf("rounds         : %d\n", rounds);
	printf("total_allocs   : %d\n", total_ops);
	printf("total_frees    : %d\n", total_ops);
	printf("total_us_est   : %d\n", total_us);
	printf("avg_us_est     : %d\n", avg_us);

	return 0;
}
