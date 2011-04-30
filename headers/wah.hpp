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
	static const word_t FILL_FV   = 0xC000000000000000LLU;
	static const word_t FILL_0    = 0x8000000000000000LLU;
	static const word_t FILL_1    = 0xC000000000000000LLU;
	static const word_t FILL_BITS = 0x3FFFFFFFFFFFFFFFLLU;
	static const word_t DATA_BITS = 0x7FFFFFFFFFFFFFFFLLU;

	static inline bool isfill (word_t w) { return w & FILL_FLAG; }
	static inline bool isfill0(word_t w) { return (w & FILL_FV) == FILL_0; }
	static inline bool isfill1(word_t w) { return (w & FILL_FV) == FILL_1; }
	static inline bool fillval(word_t w) { return w & FILL_VAL; }
	static inline bool fillcnt(word_t w) { return w & FILL_BITS; }
	
	static inline bool samefill(word_t fw, word_t lw) { return isfill(fw) && (fw & FILL_VAL) == ((lw << 1) & FILL_VAL); }

	static inline bool allones(word_t w) { return (w & DATA_BITS) == DATA_BITS; }
	static inline bool allzero(word_t w) { return (w & DATA_BITS) == 0; }
};


#endif