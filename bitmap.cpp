#include "headers/bitmap.hpp"


bitmap::bitmap(mmf* file, u8 page) : file(file), index_page(file->get_page(page))
{	
	int* h = (int*) index_page;
	std::cout << std::hex << h[0] << std::endl;
}

bitmap::~bitmap()
{
	close();
}

void bitmap::close()
{
	file->flush(index_page);
}
