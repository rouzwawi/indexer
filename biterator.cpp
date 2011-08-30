#include "headers/biterator.hpp"

#include <algorithm>


biterator::biterator(mmf& file, u4 page) : file(file) { init(page); }
biterator::~biterator() {}
void biterator::init(u4 page)
{
	load_page(page);
	length = current_page.length();
	iterated = 0;

	// first word is always a fill word, read it
	read_fill();
}

u8 biterator::next()
{
	if (!fills && !ltrls) read_fill();

	u8 res = 0;
	if (fills) {
		fills--;
		res = fillv ? wah::DATA_BITS : U8(0);
	} else if (ltrls) {
		ltrls--;
		res = current_page_data[read_words++];
	}
	iterated += wah::WORD_LENGTH;
	return res;
}

void biterator::skip_words(u4 words)
{
	std::cout << std::dec;
	while (words > 0) {
		std::cout << "." << words << std::endl;
		std::cout << "rw @ " << read_words << std::endl;
		u4 skip;

		// skip fills (will not progress read pos in page)
		skip = std::min(words, fills);
		std::cout << "skip " << skip << " fills of " << fills << std::endl;
		fills -= skip;
		words -= skip;
		iterated += skip * wah::WORD_LENGTH;

		// skip literals (progresses read pos in page)
		skip = std::min(words, ltrls);
		skip = std::min(skip, BM_DATA_WORDS - read_words); // but not past page end
		std::cout << "skip " << skip << " ltrls of " << ltrls << std::endl;
		ltrls -= skip;
		words -= skip;
		read_words += skip;
		iterated += skip * wah::WORD_LENGTH;
		std::cout << "rw @ " << read_words << std::endl;

		// done skipping or is bitmap exhausted?
		if (words == 0 || !has_next()) break;

		// end of page? load next page
		if (read_words == BM_DATA_WORDS) {
			std::cout << "next page " << current_page.next_page() << std::endl;
			load_page(current_page.next_page());
		}

		// exhausted fill word? read next
		if (!fills && !ltrls) read_fill();
	}

	std::cout << "done skipping" << std::endl;
}

void biterator::load_page(u4 page)
{
	void* current_page_ptr = file.get_page(page);
	current_page = bitmap_page(current_page_ptr);
	current_page_data = current_page.data;
	read_words = 0;
}

void biterator::read_fill()
{
	wah::word_t current_fill = current_page_data[read_words];

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

// main entry of evaluation algo
u8 boperator::operate(callback cb, skip_fill_value skip_val)
{
	cb(U8(256), 0);
}