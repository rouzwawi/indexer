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


struct fill_state {
	fill_state() : fillv(0), fills(0), ltrls(0) {}

	bool fillv;
	u4   fills;
	u4   ltrls;

	inline bool empty() { return !fills && !ltrls; }
	inline void read(const wah::word_t& fw)  {
		fillv = wah::fillval(fw);
		fills = wah::fillcnt(fw);
		ltrls = wah::ltrlcnt(fw);
	}
	inline void set(bool v, u4 f, u4 l)  {
		fillv = v;
		fills = f;
		ltrls = l;
	}
};

class noderator
{
private:
	noderator& operator=(const noderator&);

protected:
	noderator() : length(0), iterated(0) {}
	virtual ~noderator() {}

	// iteration status
	u8 length;
	u8 iterated;

	fill_state fls;

	// tree structure
	list<noderator*> children;

public:
	bool has_next() { return iterated < length; }
	u8 bits() { return length; }
	u8 last_word_mask() { return (U8(1) << (length % wah::WORD_LENGTH)) - 1; }
	const list<noderator*>& c() { return children; }
	const fill_state& s() { return fls; }
	
	virtual u8 next() = 0;
	virtual void skip_words(u4 words) = 0;
	virtual void prep_fill() = 0;

	friend class biterator;
	friend class boperator;
};


class biterator : public noderator
{
private:
	mmf&  file;
	u4    read_words;

	bitmap_page current_page;
	wah::word_t* current_page_data;

public:
	biterator(mmf& file, u4 page) : file(file) { init(page); }
	~biterator() {}

	virtual u8 next();
	virtual void skip_words(u4 words);
	virtual void prep_fill();

private:
	void init(u4 page);
	void maybe_next_page();
	void load_page(u4 page);
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

	virtual u8 next();
	virtual void skip_words(u4 words);
	virtual void prep_fill();

	u8 operate(callback cb, skip_fill_value skip_val=SKIPNONE);

private:
	void init();
};

namespace tmplt {

struct any {};
struct bol {};
struct var {};
struct zer {};

#define bchk(n, logic) (n->s().fillv == logic)
#define vchk(n, what) (n->s().what)
#define zchk(n, what) (n->s().what == 0)

template <class v, class f, class l> struct op_case {};
template <class v0, class f0, class l0, class v1, class f1, class l1> struct op_case_asym {};

/* assymetric cases will test with swapped operator opositions */

template <> struct op_case_asym <bol, var, any, any, any, any> { // assymetric
	static inline bool is(bool bool0, noderator* n0, noderator* n1) {
		return
			(bchk(n0,bool0) && vchk(n0,fills)) ||
			(bchk(n1,bool0) && vchk(n1,fills));
	}
};
template <> struct op_case <bol, var, any> {
	static inline bool is(bool bool0, noderator* n0, noderator* n1) {
		return
			bchk(n0,bool0) && vchk(n0,fills) && bchk(n1,bool0) && vchk(n1,fills);
	}
};
template <> struct op_case_asym <bol, var, any, any, zer, var> { // assymetric
	static inline bool is(bool bool0, noderator* n0, noderator* n1) {
		return
			(bchk(n0,bool0) && vchk(n0,fills) && zchk(n1,fills) && vchk(n1,ltrls)) ||
			(bchk(n1,bool0) && vchk(n1,fills) && zchk(n0,fills) && vchk(n0,ltrls));
	}
};
template <> struct op_case <any, zer, var> {
	static inline bool is(noderator* n0, noderator* n1) {
		return
			(zchk(n0,fills) && vchk(n0,ltrls) && zchk(n1,fills) && vchk(n1,ltrls));
	}
};

}

#endif