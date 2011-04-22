#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>

#include "wah.hpp"
#include "bitmap.hpp"

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

void mm(char* f)
{
	using namespace boost::interprocess;
	try{
		// Create a file mapping.
		file_mapping m_file(f, read_write);

		// Map the whole file in this process
		mapped_region region
			(m_file				//What to map
			,read_write			//Map it as read-write
			);

		// Get the address of the mapped region
		char* addr = (char*) region.get_address();

		// little endian on OSX and Linux
		int* i = (int*)addr;
		long* l = (long*)addr;
		std::cout << i[0] << std::endl; // will print 1024
		std::cout << l[0] << std::endl; // will print 1024

	}
	catch(interprocess_exception &ex){
		std::remove("file.bin");
		std::cout << ex.what() << std::endl;
	}
}

int main(int argc, const char* argv[])
{
	Bitmap bm(argv[1]);
	
	return 0;
}