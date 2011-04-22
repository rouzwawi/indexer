#include "typedefs.hpp"
#include "mmf.hpp"

#ifndef BITMAP_HPP
#define BITMAP_HPP


using namespace boost::interprocess;

class Bitmap
{
	
private:
	mapped_region	index_region;
	char*			index_page;

public:
	Bitmap(const file_mapping& file_m, u8 page);
	~Bitmap();
	
	void close();
	
};


#endif