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
		if (skip) {
			std::cout << "bit skip " << skip << " fills of " << fls.fills << std::endl;
			fls.fills -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
		}

		// skip literals (progresses read pos in page)
		skip = std::min(words, fls.ltrls);
		skip = std::min(skip, BM_DATA_WORDS - read_words); // not past page end
		if (skip) {
			std::cout << "bit skip " << skip << " ltrls of " << fls.ltrls << std::endl;
			fls.ltrls -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
			read_words += skip;
			std::cout << "rw @ " << read_words << std::endl;
		}

		// done skipping or is bitmap exhausted?
		if (words == 0 || !has_next()) break;

		// end of page? load next page
		if (read_words == BM_DATA_WORDS) {
			std::cout << "bit next page " << current_page.next_page() << std::endl;
			load_page(current_page.next_page());
		}

		// exhausted fill word? read next
		if (fls.empty()) prep_fill();
	}

	std::cout << "done skipping biterator" << std::endl;
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

	std::cout << std::dec << "fill v " << fls.fillv << " f " << fls.fills
				<< " l " << fls.ltrls << std::endl << std::endl;
}

void boperator::init()
{
	children.push_back(&op0); children.push_back(&op1);
	length = std::min(op0.length, op1.length);

	prep_fill();
}

u8 boperator::next()
{
	if (fls.empty()) prep_fill();

	u8 res = 0;
	if (fls.fills) {
		fls.fills--;
		res = fls.fillv ? wah::DATA_BITS : U8(0);
	} else if (fls.ltrls) {
		fls.ltrls--;
		switch (ope) {
			case A:
				res = op0.next() & op1.next();
				break;
			case O:
				res = op0.next() | op1.next();
				break;
		}
	}
	iterated += wah::WORD_LENGTH;
	return res;
}

void boperator::skip_words(u4 words)
{
	while (words > 0) {
		std::cout << ".." << words << std::endl;
		u4 skip;

		// skip fills
		skip = std::min(words, fls.fills);
		if (skip) {
			std::cout << "bop skip " << skip << " fills of " << fls.fills << std::endl;
			fls.fills -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
		}

		// skip literals (progresses read of operands)
		skip = std::min(words, fls.ltrls);
		if (skip) {
			std::cout << "bop skip " << skip << " ltrls of " << fls.ltrls << std::endl;
			fls.ltrls -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
			op0.skip_words(skip);
			op1.skip_words(skip);
		}

		// exhausted fill word? read next
		if (fls.empty()) prep_fill();

		// done skipping or is bitmap exhausted?
		if (words == 0 || !has_next()) break;
	}

	std::cout << "done skipping boperator" << std::endl;
}

// read about details in scratch file: # biterator flow data manipulation
void boperator::prep_fill()
{
	using namespace tmplt;

	if (op0.fls.empty()) op0.prep_fill();
	if (op1.fls.empty()) op1.prep_fill();

	if (ope == 0) {
		bool logic = ope == A ? false : true;
		u4 n = 0;
		u4 u = 0;

		if (op_case_asym<b<true>, VAR, DCR, DCR, DCR, DCR>::is(&op0, &op1)) {
			n = max(op0.fls.fills, op1.fls.fills);
			assert(u==0);
			op0.skip_words(n);
			op1.skip_words(n);
		} else
		if (op_case<b<false>, VAR, DCR>::is(&op0, &op1)) {
			n = min(op0.fls.fills, op1.fls.fills);
			assert(u==0);
			logic = !logic;
			op0.skip_words(n);
			op1.skip_words(n);
		} else
		if (op_case_asym<b<false>, VAR, DCR, DCR, i<0>, VAR>::is(&op0, &op1)) {
			assert(n==0);
			if (op0.fls.fills)
				u = min(op0.fls.fills, op1.fls.ltrls);
			else
				u = min(op1.fls.fills, op0.fls.ltrls);
		} else
		if (op_case<DCR, i<0>, VAR>::is(&op0, &op1)) {
			assert(n==0);
			u = min(op0.fls.ltrls, op1.fls.ltrls);
		}

		fls.set(logic, n, u);
	}
	// TODO: and xor ...
	
	std::cout << std::dec << "bopfill v " << fls.fillv << " f " << fls.fills
				<< " l " << fls.ltrls << std::endl << std::endl;
}

// main entry of evaluation algo
u8 boperator::operate(callback cb, skip_fill_value skip_val)
{
	//prep_fill();
	cb(U8(256), 0);
	// remember that thing about operating on stack vars
	// instead of cascading virtual method calls
	return U8(0);
}
