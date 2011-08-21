#include "headers/biterator.hpp"

#include <algorithm>


biterator::biterator(bitmap& bitmap) : file(bitmap.file) { init(bitmap); }
biterator::biterator(mmf& file, u4 page) : file(file) { bitmap bm(file, page); init(bm); }
void biterator::init(bitmap& bitmap)
{
	length = bitmap.length;
	iterated = 0;
	read_words = 0;

	load_page(bitmap.first_page_num);
	
	// first word is always a fill word
	read_fill();
}

biterator::~biterator()
{
	close();
}

u8   biterator::next()
{
	if (!fills && !ltrls) read_fill();

	u8 res = 0;
	if (fills) {
		fills--;
		res = fillv ? wah::DATA_BITS : U8(0);
	} else if (ltrls) {
		ltrls--;
		res = current_page[read_words++];
	}
	iterated += wah::WORD_LENGTH;
	return res;
}

void biterator::close()
{}

void biterator::load_page(u4 page)
{
	current_page_ptr = file.get_page(page);
	char* data_0 = ((char*) current_page_ptr) + BM_DATA_OFFSET;
	current_page = (wah::word_t*) data_0;
}

void biterator::read_fill()
{
	wah::word_t current_fill = current_page[read_words];

	if (!wah::isfill(current_fill)) {
		ltrls += 1;
		return;
	}

	fillv = wah::fillval(current_fill);
	fills = wah::fillcnt(current_fill);
	ltrls = wah::ltrlcnt(current_fill);
	read_words++;

	std::cout << std::dec << "fill v " << fillv << " f " << fills << " l " << ltrls << std::endl << std::endl;
}
