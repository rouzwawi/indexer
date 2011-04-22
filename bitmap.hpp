#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>

//#include <boost/interprocess/mapped_region.hpp>

#ifndef BITMAP_HPP
#define BITMAP_HPP

using namespace boost::interprocess;

class Bitmap
{
	
private:
	static const uint HEADER_SIZE = 16 * 1024; // 16KiB
	
	mapped_region	header_region;
	char*			header;

public:
	Bitmap(const char* file);
	~Bitmap();
	
	void close();
	
};

#endif