#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/lexical_cast.hpp>

#include <map>
#include <list>
#include <iterator>

#include "headers/typedefs.hpp"

#include "headers/sha1.hpp"
#include "headers/wah.hpp"

#include "headers/fs.hpp"
#include "headers/mmf.hpp"
#include "headers/bitmap.hpp"
#include "headers/biterator.hpp"

#define LINES 64


using namespace std;
using namespace boost::interprocess;
using boost::lexical_cast;

void wah()
{
	std::cout << wah::WORD_LENGTH << std::endl;
	std::cout << std::hex << wah::DATA_BITS << std::endl;
	std::cout << std::hex << wah::FILL_FLAG << std::endl;
	std::cout << std::hex << wah::FILL_VAL  << std::endl;
	std::cout << std::endl;

	std::cout << wah::isfill(0x7FFFFFFFFFFFFFFFLLU) << std::endl;
	std::cout << wah::isfill(0x8FFFFFFFFFFFFFFFLLU) << std::endl;
	std::cout << wah::fillval(0x8FFFFFFFFFFFFFFFLLU) << std::endl;
	std::cout << wah::fillval(0xCFFFFFFFFFFFFFFFLLU) << std::endl;
}

void test_callback(u8 word, u8 mask) { cout << word << endl; }

int test_itop()
{
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
	test_itop();

	mmf f(argv[1]);

	if (f.allocated_pages() == 0)
		fs::init(f);

	fs fileSystem(f);

	//u4 index_page_0 = lexical_cast<u4>(argv[2]);
	//bitmap bitmap(data_file, index_page_0);

	std::string x;
	std::string y;
	while (cin >> x) {
		if (x == "q") exit(0);

		if (x == "a") {
			cin >> y;
			u4 page;
			if(fileSystem.has_file(y.c_str())) {
				cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
			} else {
				page = fileSystem.create_file(y.c_str());
				bitmap::init(f, page);
				cout << "allocated '" << y << "' @ page " << hex << page << endl;
			}
			bitmap bm(f, page);
			u8 bits[] = {U8(0x7FFFFFFFFFFFFFFF), U8(0x7FFFFFFFFFFFFFFF), U8(0xAAA), U8(0xCCC), U8(0xFFF)};
			for (int v=0;v<16;v++)
				bm.append(bits, 327);
			continue;
		}

		if (x == "i") {
			cin >> y;
			u4 page;
			if(fileSystem.has_file(y.c_str())) {
				cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
			} else {
				cout << "file '" << y << "' not found...abort iteration" << endl;
				continue;
			}
			biterator it(f, page);
			while (it.has_next()) {
				cout << hex << it.next();
			}
			cout << it.last_word_mask() << endl;
			cout << "EOF" << endl;
			
			op op0 = AND;
			boperator bop(op0, it, it);
			const list<noderator*>& c = bop.c();
			for(list<noderator*>::const_iterator ct = c.begin(); ct != c.end(); ++ct)
				cout << *ct << endl;
			bop.operate(&test_callback);
			
			continue;
		}

		if (x == "s") {
			cin >> y;
			u4 page;
			if(fileSystem.has_file(y.c_str())) {
				cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
			} else {
				cout << "file '" << y << "' not found...abort iteration" << endl;
				continue;
			}
			biterator it(f, page);
			
			it.skip_words(600);
			cout << "Done skipping" << endl;
			continue;
		}

		if (x == "w") {
			cin >> y;
			
			if(fileSystem.has_file(y.c_str())) {
				u4 file_page = fileSystem.get_file_page(y.c_str());
				int* ptr = (int*) f.get_page(file_page);
				int* p;
				for (int x=0;x<PAGE_SIZE/sizeof(int);x++) {
					p = ptr + x;
					for (int y=0;y<(1<<20)-1;y++) {
						(*p)++;
					}
				}
				cout << "wrote to file '" << y << "' @ page " << hex << file_page << endl;
			} else {
				cout << "file '" << y << "' does not exists" << endl;
			}
			continue;
		}

		u4 page;
		stringstream ss;
		ss << hex << x;
		ss >> page;
		cout << "page " << dec << page << endl;
		char* p = (char*) f.get_page(page);
		for (int i=0; i<LINES; i++) {
			for (int j=0; j<64; j++) {
				int v = ((int)p[i*64 + j] & 0xFF);
				if (j % 4 == 0) cout << " ";
				cout << hex << (v<0x10?"0":"") << v;
			}
			cout << endl;
		}
	}
	
	return 0;
}