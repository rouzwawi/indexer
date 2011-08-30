#include <iterator>

#include "FastDelegate.h"

#include "typedefs.hpp"
#include "bitmap.hpp"
#include "op.hpp"
#include "wah.hpp"

#ifndef BITERATOR_H
#define BITERATOR_H


using namespace boost::interprocess;

// does not conform to starands c++ iterators
class biterator
{
	
private: // fields
	mmf&  file;

	u8    length;
	u8    iterated;
	u4    read_words;

	// fill status
	bool  fillv;
	u4    fills;
	u4    ltrls;

	bitmap_page current_page;

	wah::word_t* current_page_data;

public:
	biterator(mmf& file, u4 page);
	~biterator();

	bool has_next() { return iterated < length; }
	u8   last_word_mask() { return (U8(1) << (length % wah::WORD_LENGTH)) - 1; }

	u8   next();
	void skip_words(u4 words);

private:
	void init(u4 page);
	void load_page(u4 page);
	void read_fill();

	friend class boperator;
};


// === Operator combinations of iterators
// biterator o biterator
// ~ biterator

enum skip_fill_value { SKIPNONE, SKIP0, SKIP1 };

using namespace fastdelegate;
class boperator {
	
private:
	op ope;
	biterator it0, it1;
public:
	//typedef void (*callback) (u8, u8);
	typedef FastDelegate2<u8, u8> callback;

	boperator(op ope, biterator it0, biterator it1) : ope(ope), it0(it0), it1(it1) {}
	boperator(op ope, const boperator& bop) : ope(ope), it0(bop.it0), it1(bop.it1) {}
	boperator(const boperator& bop) : ope(bop.ope), it0(bop.it0), it1(bop.it1) {}

	u8 operate(callback cb, skip_fill_value skip_val=SKIPNONE);
};

#endif