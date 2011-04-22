#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>

#include "typedefs.hpp"

#ifndef MMF_H
#define MMF_H


using namespace boost::interprocess;

class mmf {

public:
	static void init_region(mapped_region& region, const file_mapping& file_m, u8 page)
	{
		init_region(region, file_m, page, 1);
	}
	
	static void init_region(mapped_region& region, const file_mapping& file_m, u8 page, u4 pages)
	{
		std::size_t page_size = mapped_region::get_page_size();
		mapped_region new_region(file_m, read_write, page * page_size, pages * page_size);
		region.swap(new_region);
	}

};


#endif