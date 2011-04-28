#include "typedefs.hpp"


#ifndef WAH_H
#define WAH_H


class wah
{
public:
	typedef u8 word_t;

	static const int WORD_LENGTH = 64 - 1;

	static const word_t FILL_FLAG = 0x8000000000000000LLU;
	static const word_t FILL_VAL  = 0x4000000000000000LLU;
	static const word_t FILL_BITS = 0x3FFFFFFFFFFFFFFFLLU;
	static const word_t DATA_BITS = 0x7FFFFFFFFFFFFFFFLLU;

	static inline bool isfill(word_t w) { return w & FILL_FLAG; }
	static inline bool fillvl(word_t w) { return w & FILL_VAL; }
	static inline bool fillct(word_t w) { return w & FILL_BITS; }

	static inline bool allones(word_t w) { return (w & DATA_BITS) == DATA_BITS; }
	static inline bool allzero(word_t w) { return (w & DATA_BITS) == 0; }

	static inline word_t increment(word_t w)        { return w + 1; }
	static inline word_t increment(word_t w, int n) { return w + n; }
};


#endif