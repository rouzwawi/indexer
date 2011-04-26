#include "typedefs.hpp"
#include "mmf.hpp"


#ifndef BITMAP_H
#define BITMAP_H


using namespace boost::interprocess;

class bitmap
{
	
private: // fields
	mmf&  file;
	u4    index_page;
	void* index_page_ptr;

public:
	bitmap(mmf& file, u8 page);
	~bitmap();
	
	void close();
	
};


#endif