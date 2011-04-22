#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/lexical_cast.hpp>

#include "headers/typedefs.hpp"
#include "headers/wah.hpp"
#include "headers/bitmap.hpp"

using namespace std;
using namespace boost::interprocess;
using boost::lexical_cast;

void wah()
{
	std::cout << WAH::WORD_LENGTH << std::endl;
	std::cout << std::hex << WAH::DATA_BITS << std::endl;
	std::cout << std::hex << WAH::FILL_FLAG << std::endl;
	std::cout << std::hex << WAH::FILL_VAL  << std::endl;
	std::cout << std::endl;

	std::cout << WAH::increment(0x8000000000000001) << std::endl;
	std::cout << WAH::increment(0xC000000000000001) << std::endl;

	std::cout << WAH::increment(0x8000000000000001, 4) << std::endl;
	std::cout << WAH::increment(0xC000000000000001, 4) << std::endl;

	std::cout << WAH::isfill(0x7FFFFFFFFFFFFFFF) << std::endl;
	std::cout << WAH::isfill(0x8FFFFFFFFFFFFFFF) << std::endl;
	std::cout << WAH::fillvl(0x8FFFFFFFFFFFFFFF) << std::endl;
	std::cout << WAH::fillvl(0xCFFFFFFFFFFFFFFF) << std::endl;
}

int main(int argc, const char* argv[])
{
	cout << "page_size " << mapped_region::get_page_size() << endl;
	
	file_mapping data_file(argv[1], read_write);
	u8 index_page_0 = lexical_cast<u8>(argv[2]);
	Bitmap bitmap(data_file, index_page_0);
	
	return 0;
}