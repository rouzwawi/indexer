#include <map>
#include <algorithm>
#include "typedefs.h"


#ifndef MMF_H
#define MMF_H

#define MMF_RAMP_FILES 7
#define MMF_PAGE_SIZE 4096
#define MMF_PAGES_PER_REGION 2048

#define MMF_REGION_SIZE (MMF_PAGES_PER_REGION * MMF_PAGE_SIZE)

// implies that first file has 1 region, thus 2048 pages, size 8MiB


/**********************************************
 * sizes    as size_t
 * adresses as char*
 * pages    as u4
 * regions  as u4
 * files    as int
 **********************************************/


using namespace std;

class mmf {

private: // fields
   std::map<int, int> files;      // file descriptors
   std::map<u4, void*> regions;   // region_address -> region
   const std::string            data_file;
   const std::string            data_index;

   void* index_ptr;
   u4* next_empty_page;

public:
   mmf(const char* data_file);
   ~mmf();

   u4 allocated_pages();
   u4 allocate_page();
   void* get_page(u4 page);
   void flush(u4 page);

   static size_t file_size(int file)      { return file_regions(file) * MMF_REGION_SIZE; }

   static size_t region_offset(u4 page)   { return MMF_PAGE_SIZE * (page % MMF_PAGES_PER_REGION); }
   static u4     file_regions(int file)   { return 1 << min(MMF_RAMP_FILES, file); }
   static u4     regions_up_to(int file)  { return file_regions(min(MMF_RAMP_FILES, file)) - 1 + max(0,file-MMF_RAMP_FILES) * file_regions(MMF_RAMP_FILES); }

   static u4     region_addr(u4 page)     { return page / MMF_PAGES_PER_REGION; }
   static int    file_addr(u4 region)     { for (int f=0;1;f++) if (regions_up_to(f) > region) return f-1; }

private:
   bool is_region_mapped(u4 region);
   void map_region(u4 region);
   void map_file(int file);

   void read_index();
   void write_index();

};


#endif
