#include <stdio.h>

#include "mmf.h"


int main(int argc, const char* argv[]) {
  printf("hello world\n");

  mmf memfile("mmap-test.dat");
  int* addr = (int*) memfile.get_page(0);

  for (int i = 0; i < 1024; i++) {
    addr[i] = i;
  }

  return 0;
}
