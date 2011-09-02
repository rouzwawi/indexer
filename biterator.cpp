#include "headers/biterator.hpp"

#include <algorithm>


void biterator::init(u4 page)
{
	load_page(page);
	length = current_page.length();

	// first word is always a fill word, read it
	prep_fill();
}

u8 biterator::next()
{
	if (fls.empty()) prep_fill();

	u8 res = 0;
	if (fls.fills) {
		fls.fills--;
		res = fls.fillv ? wah::DATA_BITS : U8(0);
	} else if (fls.ltrls) {
		fls.ltrls--;
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
		skip = std::min(words, fls.fills);
		std::cout << "skip " << skip << " fills of " << fls.fills << std::endl;
		fls.fills -= skip;
		words -= skip;
		iterated += skip * wah::WORD_LENGTH;

		// skip literals (progresses read pos in page)
		skip = std::min(words, fls.ltrls);
		skip = std::min(skip, BM_DATA_WORDS - read_words); // but not past page end
		std::cout << "skip " << skip << " ltrls of " << fls.ltrls << std::endl;
		fls.ltrls -= skip;
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
		if (fls.empty()) prep_fill();
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

void biterator::prep_fill()
{
	wah::word_t current_fill = current_page_data[read_words];

	if (!wah::isfill(current_fill)) {
		fls.ltrls += 1;
		return;
	}

	fls.read(current_fill);
	read_words++;

	std::cout << std::dec << "fill v " << fls.fillv << " f " << fls.fills << " l " << fls.ltrls << std::endl << std::endl;
}

void boperator::init()
{
	children.push_back(&op0); children.push_back(&op1);
	length = std::min(op0.length, op1.length);

	prep_fill();
}

void boperator::skip_words(u4 words)
{
	
}

// read about details in scratch file: # biterator flow data manipulation
void boperator::prep_fill()
{
	if (op0.fls.empty()) op0.prep_fill();
	if (op1.fls.empty()) op1.prep_fill();

	if (ope == A) {
		u4 n = 0;
		if (op0.fls.fillv == false && op0.fls.fills)
			n = op0.fls.fills;
		if (op1.fls.fillv == false && op1.fls.fills && op1.fls.fills > n)
			n = op1.fls.fills;
		fls.set(false, n, 0);
		op0.skip_words(n);
		op1.skip_words(n);
	} else if (ope == O) {
		u4 n = 0;
		if (op0.fls.fillv == true && op0.fls.fills)
			n = op0.fls.fills;
		if (op1.fls.fillv == true && op1.fls.fills && op1.fls.fills > n)
			n = op1.fls.fills;
		fls.set(true, n, 0);
		op0.skip_words(n);
		op1.skip_words(n);
	} // xor
	
	std::cout << std::dec << "bopfill v " << fls.fillv << " f " << fls.fills << " l " << fls.ltrls << std::endl << std::endl;
}

// main entry of evaluation algo
u8 boperator::operate(callback cb, skip_fill_value skip_val)
{
	prep_fill();
	cb(U8(256), 0);
	// remember that thing about operating on stack vars
	// instead of cascading virtual method calls
}