#include <list>

#include "FastDelegate.h"

#include "typedefs.hpp"
#include "bitmap.hpp"
#include "op.hpp"
#include "wah.hpp"

#ifndef BITERATOR_H
#define BITERATOR_H


using namespace boost::interprocess;
using namespace std;

class noderator
{
private:
	noderator& operator=(const noderator&);

protected:
	noderator() : length(0), iterated(0) {}
	virtual ~noderator() {};

	// fill status
	bool  fillv;
	u4    fills;
	u4    ltrls;

	// iteration status
	u8    length;
	u8    iterated;

	// tree structure
	list<noderator*> children;

public:
	virtual bool has_next() { return iterated < length; }

	virtual u8 next() = 0;
	virtual void skip_words(u4 words) = 0;

	u8 last_word_mask() { return (U8(1) << (length % wah::WORD_LENGTH)) - 1; }

	virtual const list<noderator*>& c() { return children; }
};

// does not conform to starands c++ iterators
class biterator : public noderator
{
private:
	mmf&  file;
	u4    read_words;

	bitmap_page current_page;
	wah::word_t* current_page_data;

public:
	biterator(mmf& file, u4 page) : file(file) { init(page); };
	~biterator() {};

	virtual u8 next();
	virtual void skip_words(u4 words);

private:
	void init(u4 page);
	void load_page(u4 page);
	void read_fill();
};


// === Operator combinations of iterators
// biterator o biterator (AND, OR, XOR)
// ~ biterator (NOT)

enum skip_fill_value { SKIPNONE, SKIP0, SKIP1 };

using namespace fastdelegate;
class boperator : public noderator
{
private:
	op ope;
	noderator& op0;
	noderator& op1;

public:
	//typedef void (*callback) (u8, u8);
	typedef FastDelegate2<u8, u8> callback;

	boperator(op ope, noderator& op0, noderator& op1) : ope(ope), op0(op0), op1(op1) { init(); }
	boperator(op ope, const boperator& bop) : ope(ope), op0(bop.op0), op1(bop.op1) { init(); }
	boperator(const boperator& bop) : ope(bop.ope), op0(bop.op0), op1(bop.op1) { init(); }
	~boperator() {}

	virtual u8 next() {};
	virtual void skip_words(u4 words) {};

	u8 operate(callback cb, skip_fill_value skip_val=SKIPNONE);

private:
	void init() { children.push_back(&op0); children.push_back(&op1); }
};

#endif