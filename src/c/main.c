#include <stdio.h>

void presage_link(char *);

int main(int, char**) {
    printf("Calling presage_link…\n");
    presage_link("devicename"); 
    printf("Finished presage_link…\n");
    return 0;
}
