#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>

#include <map>

#include "typedefs.hpp"

#ifndef MMF_H
#define MMF_H

#define PAGES_PER_REGION 2048
#define REGIONS_PER_FILE 256


using namespace boost::interprocess;

class mmf {

private: // fields
	std::map<u4, mapped_region*> regions;	// region_address -> region
	const file_mapping&          file_m;

public:
	// TODO: take filename and handle file allocation
	mmf(const file_mapping& file_m);
	~mmf();

	char* get_page(u4 page);
	void flush(char* page);

	static std::size_t page_size()   { return mapped_region::get_page_size();      }
	static std::size_t region_size() { return PAGES_PER_REGION * mmf::page_size(); }

private:
	bool is_region_mapped(u4 region_addr);
	void map_region(u4 region_addr);

	u4 region_addr(u4 page) { return page / PAGES_PER_REGION; }

};


#endif