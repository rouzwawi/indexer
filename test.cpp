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

int test_addr_calcs()
{
	assert(0x00001000 == mmf::page_size());
	assert(0x00800000 == mmf::region_size());

	assert(0x00800000 == mmf::file_size(0));
	assert(0x01000000 == mmf::file_size(1));
	assert(0x02000000 == mmf::file_size(2));
	assert(0x04000000 == mmf::file_size(3));
	assert(0x08000000 == mmf::file_size(4));
	assert(0x10000000 == mmf::file_size(5));
	assert(0x20000000 == mmf::file_size(6));
	assert(0x40000000 == mmf::file_size(7));
	assert(0x40000000 == mmf::file_size(8));

	assert(  1 == mmf::file_regions(0));
	assert(  2 == mmf::file_regions(1));
	assert(  4 == mmf::file_regions(2));
	assert(  8 == mmf::file_regions(3));
	assert( 16 == mmf::file_regions(4));
	assert( 32 == mmf::file_regions(5));
	assert( 64 == mmf::file_regions(6));
	assert(128 == mmf::file_regions(7));
	assert(128 == mmf::file_regions(8));

	assert(  0 == mmf::regions_up_to(0));
	assert(  1 == mmf::regions_up_to(1));
	assert(  3 == mmf::regions_up_to(2));
	assert(  7 == mmf::regions_up_to(3));
	assert( 15 == mmf::regions_up_to(4));
	assert( 31 == mmf::regions_up_to(5));
	assert( 63 == mmf::regions_up_to(6));
	assert(127 == mmf::regions_up_to(7));
	assert(255 == mmf::regions_up_to(8));

	assert( 0 == mmf::file_addr(  0));
	assert( 1 == mmf::file_addr(  1));
	assert( 1 == mmf::file_addr(  2));
	assert( 2 == mmf::file_addr(  3));
	assert( 2 == mmf::file_addr(  6));
	assert( 3 == mmf::file_addr(  7));
	assert( 3 == mmf::file_addr( 14));
	assert( 4 == mmf::file_addr( 15));
	assert( 4 == mmf::file_addr( 30));
	assert( 5 == mmf::file_addr( 31));
	assert( 5 == mmf::file_addr( 62));
	assert( 6 == mmf::file_addr( 63));
	assert( 6 == mmf::file_addr(126));
	assert( 7 == mmf::file_addr(127));
	assert( 7 == mmf::file_addr(254));
	assert( 8 == mmf::file_addr(255));
	assert( 8 == mmf::file_addr(382));
	assert( 9 == mmf::file_addr(383));

	return 0;
}

int main(int argc, const char* argv[])
{
	test_addr_calcs();
	
	
	file_mapping data_file(argv[1], read_write);
	
	mmf f(data_file);
	
	u4 index_page_0 = lexical_cast<u4>(argv[2]);
	//bitmap bitmap(data_file, index_page_0);
	
	string x;
	while (cin >> x) {
		if (x == "q") exit(0);
		
		//bitmap.foo();
		u4 page = lexical_cast<u4>(x);
		//int* p = (int*) f.get_page(page);
		//cout << hex << p[0] << endl;
		
		cout << "page " << page << " region " << mmf::region_addr(page) << " file " << mmf::file_addr(mmf::region_addr(page)) << endl;
	}
	
	return 0;
}