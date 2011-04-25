#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>

#include <map>
#include <algorithm>


#include "typedefs.hpp"

#ifndef MMF_H
#define MMF_H

#define RAMP_FILES 7
#define PAGE_SIZE 4096
#define PAGES_PER_REGION 2048


/**********************************************
 * sizes    as size_t
 * adresses as char*
 * pages    as u4
 * regions  as u4
 * files    as int
 **********************************************/


using namespace boost::interprocess;
using namespace std;

class mmf {

private: // fields
	std::map<int, file_mapping*> files;
	std::map<u4, mapped_region*> regions;	// region_address -> region
	const std::string            data_file;

public:
	mmf(const char* data_file);
	~mmf();

	char* get_page(u4 page);
	void flush(char* page);

	static size_t page_size()             { return PAGE_SIZE; }
	static size_t region_size()           { return PAGES_PER_REGION * page_size(); }
	static size_t file_size(int file)     { return file_regions(file) * region_size(); }

	static size_t region_offset(u4 page)  { return page % PAGES_PER_REGION * page_size(); }
	static u4     file_regions(int file)  { return 1 << min(RAMP_FILES, file); }
	static u4     regions_up_to(int file) { return file_regions(min(RAMP_FILES, file)) - 1 + max(0,file-RAMP_FILES) * file_regions(RAMP_FILES); }

	static u4     region_addr(u4 page)    { return page / PAGES_PER_REGION; }
	static int    file_addr(u4 region)    { for (int f=0;1;f++) if (regions_up_to(f) > region) return f-1; }
	

private:
	bool is_region_mapped(u4 region);
	void map_region(u4 region);
	void map_file(int file);

};


#endif