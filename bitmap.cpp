#include "headers/bitmap.hpp"


Bitmap::Bitmap(const file_mapping& file_m, u8 page)
{
	mmf::init_region(index_region, file_m, page);
	index_page = (char*) index_region.get_address();
	
	int* h = (int*) index_page;
	std::cout << std::hex << h[0] << std::endl;
}

Bitmap::~Bitmap()
{
	close();
}

void Bitmap::close()
{
	index_region.flush(0, index_region.get_size());
}
