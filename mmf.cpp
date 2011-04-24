#include <boost/lexical_cast.hpp>
#include <sys/stat.h> 
#include <stdio.h>


#include "headers/mmf.hpp"


mmf::mmf(const char* data_file) : data_file(std::string(data_file))
{
}

mmf::~mmf()
{
	// iterate all regions
	//   flush and delete regions
	//   close and delete file mappings (?)
}

char* mmf::get_page(u4 page)
{
	u4 ra = region_addr(page);
	if (!is_region_mapped(ra)) map_region(ra);

	mapped_region* region = regions[ra];
	char* addr = (char*) region->get_address();
	return addr + region_offset(page);
}

void mmf::flush(char* page)
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

	struct stat stFileInfo; 
	if(0 != stat(fn, &stFileInfo)) {
		// allocate empty file
		FILE* pFile;
		pFile = fopen(file_name.c_str(), "w");
		fseek(pFile, file_size(file)-1, SEEK_SET);
		putc(0, pFile);
		fclose(pFile);

		std::cerr << "allocated empty file " << fn << " size 0x" << hex << file_size(file) << std::endl;
	}

	if (0 != files[file]) return;

	// create file mapping
	file_mapping* file_m = new file_mapping(fn, read_write);
	files[file] = file_m;

	std::cerr << "file mapping " << fn << std::endl;
}
