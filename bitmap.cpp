#include "headers/bitmap.hpp"

#include <algorithm>


bitmap::bitmap(mmf& file, u4 page) : file(file)
{
   first_page_num = page;
   first_page_ptr = file.get_page(first_page_num);

   first_page = bitmap_page(first_page_ptr);

   // meta....
   length = first_page.length();

   // load page that contains last fill word and get reference to it
   load_fill();

   // find last page and use as current
   load_page(first_page.last_page());

   // assert that this is the last page
   assert(current_page.next_page() == 0);
}

bitmap::~bitmap()
{
   close();
}

void bitmap::append(u8* buffer, int bits)
{
   u4 lo_len = (BM_DATA_BITS - cw_offset);
   u4 hi_len = cw_offset;

   u4 words = bits / BM_DATA_BITS;
   u4 remain = bits % BM_DATA_BITS;

   // do we have a word split?
   if (hi_len) {
      // two-phase transfer of low and high parts of data

      register wah::word_t cw = *current_word;
      for (int i=0; i<words;) {
         int bulk_words = std::min(BM_DATA_WORDS - written_words, words - i);
         for (int j=0; j<bulk_words; j++) {
            u8 data = buffer[i + j];

            // phase 1 move low data to high word pos
            cw |= data << hi_len;

            // current word is now full
            full_word(cw);
            cw = 0;

            // phase 2 move high data to low word pos
            cw |= data >> lo_len;
         }
         i += bulk_words;
         full_page();
      }
      *current_word = cw;
   } else {
      // single-phase transfer of full words
      for (int i=0; i<words;) {
         int bulk_words = std::min(BM_DATA_WORDS - written_words, words - i);
         for (int j=0; j<bulk_words; j++) {
            full_word(buffer[i + j]);
         }
         i += bulk_words;
         full_page();
      }
   }

   // cw_ffset is not changed

   // handle remain bits
   if (remain) {
      u8 data = buffer[words];

      lo_len = std::min(remain, (BM_DATA_BITS - cw_offset));
      hi_len = remain - lo_len;

      *current_word |= data << cw_offset;
      cw_offset += lo_len;
      cw_offset %= BM_DATA_BITS;

      if (!cw_offset) { // word is full
         full_word(*current_word);
         full_page();
      }

      if (hi_len) { // bits for next word
         *current_word |= data >> lo_len;
         cw_offset += hi_len;
      }
   }

   length += bits;
   update_headers();
   file.flush(current_page_num);
}

void bitmap::fill(bool value, int bits)
{

}

void bitmap::close()
{
   file.flush(first_page_num);
}

inline void bitmap::full_word(wah::word_t& w)
{
   w &= wah::DATA_BITS;
   if (wah::allones(w) || wah::allzero(w)) { // compress
      if ((*last_fill_word & wah::LTRL_BITS) == 0 && wah::samefill(*last_fill_word, w)) {
         (*last_fill_word)++;
      } else {
         new_fill(wah::allones(w));
         current_word++;
         written_words++;
      }
   } else { // literal
      *current_word = w;
      wah::incr_ltrl(last_fill_word);
      current_word++;
      written_words++;
   }
}

inline void bitmap::full_page()
{
   // do we actually have a full page?
   if (written_words != BM_DATA_WORDS) return;

   // save actual offset value since a full page can occur during 2-phase transfers
   u4 cwo = cw_offset;
   cw_offset = 0;
   update_headers();

   // allocate a new page and, use as next from current and last from first
   u4 next_page = file.allocate_page();
   first_page.last_page(next_page);
   current_page.next_page(next_page);
   file.flush(first_page_num);
   file.flush(current_page_num);

   // initialize the new page and load it
   init(file, next_page, 0, 0);
   load_page(next_page);

   // restore word offset
   cw_offset = cwo;
   update_headers();
}

inline void bitmap::new_fill(bool v)
{
   // the current word is to be promoted to a fill word
   if (v)
      *current_word = wah::FILL_1 + 1;
   else
      *current_word = wah::FILL_0 + 1;

   first_page.last_fill_page(current_page_num);
   first_page.last_fill_pos(written_words);
   load_fill();
}

inline void bitmap::load_fill()
{
   u4 last_fill_page = first_page.last_fill_page();
   u4 last_fill_pos  = first_page.last_fill_pos();

   if (last_fill_page == current_page_num) {
      last_fill_word = current_page.data + last_fill_pos;
   } else {
      bitmap_page p(file.get_page(last_fill_page));
      last_fill_word = p.data + last_fill_pos;
   }
}

inline void bitmap::init_page(u4 page)
{
   init(file, page);
}

inline void bitmap::load_page(u4 page)
{
   current_page_num = page;
   current_page_ptr = file.get_page(current_page_num);
   current_page = bitmap_page(current_page_ptr);

   // read header
   written_words = current_page.written_words();
   cw_offset = current_page.cw_offset();

   // point current word pointer to active word on page
   current_word = current_page.data + written_words;
}

inline void bitmap::update_headers()
{
   first_page.length(length);

   current_page.written_words(written_words);
   current_page.cw_offset(cw_offset);
}

void bitmap::init(mmf& file, u4 page) {
   // initiate first page of bitmap
   // last_fill_page is same page, last_page is same page
   init(file, page, page, page);
}

void bitmap::init(mmf& file, u4 page, u4 last_fill_page, u4 last_page)
{
   bool first_page = page == last_page;

   // make sure page is empty and claim it by writing magic word
   u4* ptr = (u4*) file.get_page(page);
   assert(ptr[0] == 0);
   ptr[0] = BM_MAGIC_WORD;

   bitmap_page p(ptr);

   // write header values
   p.next_page(0);
   p.written_words(first_page ? 1 : 0);
   p.cw_offset(0);
   p.length(0);
   p.last_page(last_page);
   p.last_fill_page(last_fill_page);
   p.last_fill_pos(0);

   // if first page, write initial fill word
   if (first_page) *p.data = wah::FILL_0;
}
