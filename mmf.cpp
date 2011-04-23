#include "headers/mmf.hpp"


mmf::mmf(const file_mapping& file_m) : file_m(file_m)
{
}

mmf::~mmf()
{
	// iterate all regions, flush and delete
}

char* mmf::get_page(u4 page)
{
	u4 ra = region_addr(page);
	if (!is_region_mapped(ra)) map_region(ra);

	mapped_region* region = regions[ra];
	char* addr = (char*) region->get_address();
	return addr + (page % PAGES_PER_REGION) * mmf::page_size();
}

void mmf::flush(char* page)
{
}

bool mmf::is_region_mapped(u4 region_addr)
{
	return 0 != regions[region_addr];
}

void mmf::map_region(u4 region_addr)
{
	if (is_region_mapped(region_addr)) return;

	u4 region_start = region_addr * PAGES_PER_REGION * mmf::page_size();
	mapped_region* region = new mapped_region(file_m, read_write, region_start, mmf::region_size());

	std::cout << "load region " << region_addr << " @" << region_start << std::endl;

	regions[region_addr] = region;
}
