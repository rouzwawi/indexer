#include "bitmap.hpp"

Bitmap::Bitmap(const char* file):
header_region(file_mapping(file, read_write), read_write, 0, HEADER_SIZE)
{
	header = (char*) header_region.get_address();
	
	int* h = (int*) header;
}

Bitmap::~Bitmap()
{
	close();
}

void Bitmap::close()
{
	std::cout << "close" << std::endl;
	header_region.flush(0, HEADER_SIZE);
}
