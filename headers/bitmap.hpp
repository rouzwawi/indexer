#include "typedefs.hpp"
#include "mmf.hpp"

#ifndef BITMAP_HPP
#define BITMAP_HPP


using namespace boost::interprocess;

class bitmap
{
	
private: // fields
	mmf*  file;
	char* index_page;

public:
	bitmap(mmf* file, u8 page);
	~bitmap();
	
	void close();
	
};


#endif