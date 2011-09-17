#include "headers/biterator.hpp"

#include <algorithm>


void biterator::init(u4 page)
{
	load_page(page);
	length = current_page.length();

	// first word is always a fill word, read it
	prep_fill();
}

inline u8 biterator::next()
{
	if (fls.empty()) prep_fill();

	u8 res = 0;
	if (fls.fills) {
		fls.fills--;
		res = fls.fillv ? wah::DATA_BITS : U8(0);
	} else if (fls.ltrls) {
		fls.ltrls--;
		res = current_page_data[read_words++];
		maybe_next_page();
	}
	iterated += wah::WORD_LENGTH;
	return res;
}

inline void biterator::skip_words(u4 words)
{
	while (words > 0) {
#ifdef _DEBUG
		std::cerr << "." << words << std::endl;
		std::cerr << "rw @ " << read_words << std::endl;
#endif
		u4 skip;

		// skip fills (will not progress read pos in page)
		skip = std::min(words, fls.fills);
		if (skip) {
#ifdef _DEBUG
			std::cerr << "bit skip " << skip << " fills of " << fls.fills << std::endl;
#endif
			fls.fills -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
		}

		// skip literals (progresses read pos in page)
		skip = std::min(words, fls.ltrls);
		skip = std::min(skip, BM_DATA_WORDS - read_words); // not past page end
		if (skip) {
#ifdef _DEBUG
			std::cerr << "bit skip " << skip << " ltrls of " << fls.ltrls << std::endl;
#endif
			fls.ltrls -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
			read_words += skip;
#ifdef _DEBUG
			std::cerr << "rw @ " << read_words << std::endl;
#endif
		}

		// done skipping or is bitmap exhausted?
		if (words == 0 || !has_next()) break;

		maybe_next_page();

		// exhausted fill word? read next
		if (fls.empty()) prep_fill();
	}
#ifdef _DEBUG
	std::cerr << "done skipping biterator" << std::endl;
#endif
}

inline void biterator::maybe_next_page()
{
	// end of page? load next page
	if (read_words == BM_DATA_WORDS) {
#ifdef _DEBUG
		std::cerr << "bit next page " << current_page.next_page() << std::endl;
#endif
		load_page(current_page.next_page());
	}
}

inline void biterator::load_page(u4 page)
{
	void* current_page_ptr = file.get_page(page);
	current_page = bitmap_page(current_page_ptr);
	current_page_data = current_page.data;
	read_words = 0;
}

inline void biterator::prep_fill()
{
	wah::word_t current_fill = current_page_data[read_words];

	if (!wah::isfill(current_fill)) {
		fls.ltrls += 1;
		return;
	}

	fls.read(current_fill);
	read_words++;
	maybe_next_page();

#ifdef _DEBUG
	std::cerr << std::dec << "fill v " << fls.fillv << " f " << fls.fills
				<< " l " << fls.ltrls << std::endl << std::endl;
#endif
}

void boperator::init()
{
	children.push_back(&op0); children.push_back(&op1);
	length = std::min(op0.length, op1.length);

	prep_fill();
}

inline u8 boperator::next()
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

inline void boperator::skip_words(u4 words)
{
	while (words > 0) {
#ifdef _DEBUG
		std::cerr << ".." << words << std::endl;
#endif
		u4 skip;

		// skip fills
		skip = std::min(words, fls.fills);
		if (skip) {
#ifdef _DEBUG
			std::cerr << "bop skip " << skip << " fills of " << fls.fills << std::endl;
#endif
			fls.fills -= skip;
			words -= skip;
			iterated += skip * wah::WORD_LENGTH;
		}

		// skip literals (progresses read of operands)
		skip = std::min(words, fls.ltrls);
		if (skip) {
#ifdef _DEBUG
			std::cerr << "bop skip " << skip << " ltrls of " << fls.ltrls << std::endl;
#endif
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
#ifdef _DEBUG
	std::cerr << "done skipping boperator" << std::endl;
#endif
}

// read about details in scratch file: # biterator flow data manipulation
inline void boperator::prep_fill()
{
	using namespace tmplt;

	if (op0.fls.empty()) op0.prep_fill();
	if (op1.fls.empty()) op1.prep_fill();

	if (ope == O || ope == A) {
		bool logic = ope == O;
		u4 n = 0;
		u4 u = 0;

		if (op_case_asym<bol, var, any, any, any, any>::is(logic, &op0, &op1)) {
			n = max(op0.fls.fills, op1.fls.fills);
			assert(u==0);
		} else
		if (op_case<bol, var, any>::is(!logic, &op0, &op1)) {
			n = min(op0.fls.fills, op1.fls.fills);
			assert(u==0);
			logic = !logic;
		} else 
		if (op_case_asym<bol, var, any, any, zer, var>::is(!logic, &op0, &op1)) {
			assert(n==0);
			if (op0.fls.fills)
				u = min(op0.fls.fills, op1.fls.ltrls);
			else
				u = min(op1.fls.fills, op0.fls.ltrls);
		} else
		if (op_case<any, zer, var>::is(&op0, &op1)) {
			assert(n==0);
			u = min(op0.fls.ltrls, op1.fls.ltrls);
		}

		if (n) {
			op0.skip_words(n);
			op1.skip_words(n);
		}

		// set fill state according to flow rules
		fls.set(logic, n, u);
	} else if (ope == N) {
		// TODO: implement NOT flow rules
	} else /* X */ {
		assert(false); // make sure XOR is not used
	}
	
#ifdef _DEBUG
	std::cerr << std::dec << "bopfill v " << fls.fillv << " f " << fls.fills
				<< " l " << fls.ltrls << std::endl << std::endl;
#endif
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
