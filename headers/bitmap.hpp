#include "typedefs.hpp"
#include "mmf.hpp"

#ifndef BITMAP_H
#define BITMAP_H


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