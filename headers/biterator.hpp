#include "typedefs.hpp"
#include "bitmap.hpp"
#include "op.hpp"
#include "wah.hpp"

#ifndef BITERATOR_H
#define BITERATOR_H


using namespace boost::interprocess;

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

	void* current_page_ptr;
	u4*   current_page_header;

	wah::word_t* current_page;

public:
	biterator(bitmap& bitmap);
	biterator(mmf& file, u4 page);
	~biterator();

	bool has_next() { return iterated < length; }
	u8   last_word_mask() { return (U8(1) << (length % wah::WORD_LENGTH)) - 1; }

	u8   next();

	void close();

private:
	void init(bitmap& bitmap);
	void load_page(u4 page);
	void read_fill();
};


// === Operator combinations of iterators
// biterator o biterator
// ~ biterator

template <op OP> class boperator {
};

template <> class boperator <AND> {
};
template <> class boperator <OR> {
};
template <> class boperator <XOR> {
};


#endif