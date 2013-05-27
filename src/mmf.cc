#include "mmf.h"

#include <stdio.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/mman.h>

#include <boost/lexical_cast.hpp>


void u_allocate_file(const char* filename, std::size_t size)
{
   struct stat stFileInfo;
   if (0 != stat(filename, &stFileInfo)) {
      // allocate empty file
      FILE* pFile = fopen(filename, "w");
      fseek(pFile, size-1, SEEK_SET);
      putc(0, pFile);
      fclose(pFile);

      std::cerr << "allocated empty file " << filename << " size 0x" << std::hex << size << std::endl;
   }
}

mmf::mmf(const char* data_file) : data_file(std::string(data_file)), data_index(std::string(data_file) + ".idx")
{
   read_index();
}

mmf::~mmf()
{
   // iterate all regions
   //   flush and delete regions
   //   close and delete file mappings (?)
}

u4 mmf::allocated_pages()
{
   return (*next_empty_page);
}

u4 mmf::allocate_page()
{
   u4 page = (*next_empty_page)++;
   write_index();
   return page;
}

void* mmf::get_page(u4 page)
{
   u4 ra = region_addr(page);
   if (!is_region_mapped(ra)) map_region(ra);

   char* addr = (char*) regions[ra];
   return (void*) (addr + region_offset(page));
}

void mmf::flush(u4 page)
{
}

bool mmf::is_region_mapped(u4 region)
{
   return 0 != regions[region];
}

void mmf::map_region(u4 region)
{
   if (is_region_mapped(region)) return;

   int file = file_addr(region);
   map_file(file);
   int fd = files[file];

   u4 region_start = (region - regions_up_to(file)) * MMF_REGION_SIZE;
   void* map = mmap(0, MMF_REGION_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, fd, region_start);
   if (map == MAP_FAILED) {
     perror("failed to map file region");
     exit(EXIT_FAILURE);
   }
   //mapped_region* m_region = new mapped_region(*file_m, read_write, region_start, mmf::region_size());

   regions[region] = map;

   std::cerr << "load region " << region << " in file " << file << " @ 0x" << hex << region_start << std::endl;
   std::cerr << map << std::endl;
}

void mmf::map_file(int file)
{
   std::string file_name = data_file + "." + boost::lexical_cast<std::string>(file);
   const char* fn = file_name.c_str();

   u_allocate_file(fn, file_size(file));

   if (0 != files[file]) return;

   // create file mapping
   int fd = open(fn, O_RDWR, (mode_t)0600);
   if (fd == -1) {
     perror("failed to open file");
     perror(fn);
     exit(EXIT_FAILURE);
   }
   files[file] = fd;

   std::cerr << "file mapping " << fn << std::endl;
}

void mmf::read_index()
{
   u_allocate_file(data_index.c_str(), 64);

   int fd = open(data_index.c_str(), O_RDWR, (mode_t)0600);
   if (fd == -1) {
     perror("failed to open index file");
     exit(EXIT_FAILURE);
   }
   void* map = mmap(0, 64, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
   if (map == MAP_FAILED) {
     perror("failed to map index file");
     exit(EXIT_FAILURE);
   }
   index_ptr = map;
   next_empty_page = (u4*) index_ptr;
}

void mmf::write_index()
{
   //index_region.flush(0, 64);
}
