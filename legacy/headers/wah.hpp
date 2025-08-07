#include "typedefs.hpp"


#ifndef WAH_H
#define WAH_H


class wah
{
public:
   typedef u8 word_t;

   static const int WORD_LENGTH = 64 - 1;

   static const word_t FILL_FLAG = U8(0x8000000000000000);
   static const word_t FILL_VAL  = U8(0x4000000000000000);
   static const word_t FILL_FV   = U8(0xC000000000000000);
   static const word_t FILL_0    = U8(0x8000000000000000);
   static const word_t FILL_1    = U8(0xC000000000000000);
   static const word_t FILL_BITS = U8(0x000000007FFFFFFF);
   static const word_t LTRL_BITS = U8(0x3FFFFFFF80000000);
   static const word_t DATA_BITS = U8(0x7FFFFFFFFFFFFFFF);

   static inline bool isfill (word_t w) { return  w & FILL_FLAG; }
   static inline bool isfill0(word_t w) { return (w & FILL_FV) == FILL_0; }
   static inline bool isfill1(word_t w) { return (w & FILL_FV) == FILL_1; }
   static inline bool fillval(word_t w) { return  w & FILL_VAL; }
   static inline u4   fillcnt(word_t w) { return (u4)(w & FILL_BITS); }
   static inline u4   ltrlcnt(word_t w) { return (u4)((w & LTRL_BITS) >> 31); } // (64-2)/2

   static inline bool samefill(word_t fw, word_t lw) { return isfill(fw) && (fw & FILL_VAL) == (lw & FILL_VAL); }

   static inline bool allones(word_t w) { return (w & DATA_BITS) == DATA_BITS; }
   static inline bool allzero(word_t w) { return (w & DATA_BITS) == 0; }

   static inline void incr_ltrl(word_t* w) { *w += (U8(1) << 31); }
};


#endif
