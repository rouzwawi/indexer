#include "mmf.h"

#include <stdio.h>
#include <sys/stat.h>

#include <boost/lexical_cast.hpp>


void u_allocate_file(const char* filename, std::size_t size)
{
   struct stat stFileInfo;
   if(0 != stat(filename, &stFileInfo)) {
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

   mapped_region* region = regions[ra];
   char* addr = (char*) region->get_address();
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
   file_mapping* file_m = files[file];

   u4 region_start = (region - regions_up_to(file)) * PAGES_PER_REGION * page_size();
   mapped_region* m_region = new mapped_region(*file_m, read_write, region_start, mmf::region_size());

   regions[region] = m_region;

   std::cerr << "load region " << region << " in file " << file << " @ 0x" << hex << region_start << std::endl;
}

void mmf::map_file(int file)
{
   std::string file_name = data_file + "." + boost::lexical_cast<std::string>(file);
   const char* fn = file_name.c_str();

   u_allocate_file(fn, file_size(file));

   if (0 != files[file]) return;

   // create file mapping
   file_mapping* file_m = new file_mapping(fn, read_write);
   files[file] = file_m;

   std::cerr << "file mapping " << fn << std::endl;
}

void mmf::read_index()
{
   u_allocate_file(data_index.c_str(), 64);

   file_mapping fm(data_index.c_str(), read_write);
   mapped_region mr(fm, read_write, 0, 64);
   index_region.swap(mr);
   next_empty_page = (u4*) index_region.get_address();
}

void mmf::write_index()
{
   index_region.flush(0, 64);
}
