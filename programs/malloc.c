#include <stdio.h>
#include <stdlib.h>  
#include <malloc.h>
#include <sys.h>

  
int main1()
{  
    //int *a = (int*)malloc(100);
    //int a = sbrk(0);
    //printf("end of data is : %d\n", a);
    //printf("Malloc success\n");
    //int b = sbrk(128); 
    int i;
    printf("test of malloc\n"); 
    int *c = malloc(128);
    printf("end of data after is : %d\n", c);
    int *d = malloc(256);
    printf("end of data after is : %d\n", d);
    int *f = malloc(256);
    printf("end of data after is : %d\n", f);
    free(d); 
    int *e = malloc(83); 
    printf("end of data after is : %d\n", e);

    void *array[256] = {NULL};

    for (i = 1; i < 128; ++i) {
        array[i] = malloc(i);
        printf("test of malloc on %d;\n", i);
    }
    for (i = 1; i < 128; ++i) {
        free(array[i]);
        printf("test of free on %d;\n", i);
    } 
 
    //free(c);
    //free((void*)a);
    //printf("Free success\n");
    return 0; 
}
