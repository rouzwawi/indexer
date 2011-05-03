#include "typedefs.hpp"
#include "mmf.hpp"
#include "wah.hpp"


#ifndef BITMAP_H
#define BITMAP_H

#define BM_MAGIC_WORD 0x706d7462

#define BM_DATA_BITS 63

#define BM_DATA_OFFSET 64
#define BM_DATA_WORDS 504

#define BM_HEAD_LAST_PAGE 1
#define BM_HEAD_NEXT_PAGE 2
#define BM_HEAD_WRITTEN_WORDS 3
#define BM_HEAD_CW_OFFSET 4


using namespace boost::interprocess;

class bitmap
{
	
private: // fields
	mmf&  file;
	u4    first_page;
	u4    current_page;
	u4    written_words;
	u4    cw_offset;

	void* first_page_ptr;
	void* current_page_ptr;

	u4* first_page_header;
	u4* current_page_header;

	wah::word_t* current_word;

public:
	bitmap(mmf& file, u4 page);
	~bitmap();

	void append(u8* buffer, int bits);
	void fill(bool value, int bits);

	void close();

	static void init(mmf& file, u4 page);

private:
	void full_word(wah::word_t& w);
	void full_page();

	void init_page(u4 page);
	void load_page(u4 page);
	void update_headers();

	static void init(mmf& file, u4 page, u4 last_page);
};


#endif