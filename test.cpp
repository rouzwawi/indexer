#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/lexical_cast.hpp>

#include <map>

#include "headers/typedefs.hpp"
#include "headers/wah.hpp"
#include "headers/mmf.hpp"
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
	cout << "page_size " << mmf::page_size() << endl;
	
	file_mapping data_file(argv[1], read_write);
	
	mmf f(data_file);
	
	u4 index_page_0 = lexical_cast<u4>(argv[2]);
	//bitmap bitmap(data_file, index_page_0);
	
	string x;
	do {
		//bitmap.foo();
		cin >> x;
		u4 page = lexical_cast<u4>(x);
		int* p = (int*) f.get_page(page);
		cout << hex << p[0] << endl;
	} while (x != "q");
	
	return 0;
}