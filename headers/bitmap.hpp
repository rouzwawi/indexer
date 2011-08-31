#include "typedefs.hpp"
#include "mmf.hpp"
#include "wah.hpp"


#ifndef BITMAP_H
#define BITMAP_H

#define BM_MAGIC_WORD 0x706d7462

#define BM_DATA_BITS 63

#define BM_DATA_OFFSET 64
#define BM_DATA_WORDS 504

#define BM_HEAD_NEXT_PAGE 1
#define BM_HEAD_WRITTEN_WORDS 2
#define BM_HEAD_CW_OFFSET 3

#define BM_HEAD_LENGTH_0 4
#define BM_HEAD_LENGTH_1 5

#define BM_HEAD_LAST_PAGE 6
#define BM_HEAD_LAST_FILL_PAGE 7
#define BM_HEAD_LAST_FILL_POS 8


using namespace boost::interprocess;

class bitmap_page
{
private:
	bitmap_page(){}
	bitmap_page(void* page) : page( page ), header( (u4*) page ), data( (wah::word_t*)(((char*) page) + BM_DATA_OFFSET) ) {
		assert(header[0] == BM_MAGIC_WORD);
	}

	u4 magic()			{ return header[0]; }

	u4 next_page()		{ return header[BM_HEAD_NEXT_PAGE]; }
	u4 written_words()	{ return header[BM_HEAD_WRITTEN_WORDS]; }
	u4 cw_offset()		{ return header[BM_HEAD_CW_OFFSET]; }

	u4 last_page()		{ return header[BM_HEAD_LAST_PAGE]; }
	u4 last_fill_page()	{ return header[BM_HEAD_LAST_FILL_PAGE]; }
	u4 last_fill_pos()	{ return header[BM_HEAD_LAST_FILL_POS]; }

	u8 length()			{ return ((u8)header[BM_HEAD_LENGTH_0]) | (((u8)header[BM_HEAD_LENGTH_1]) << 32); }

	void next_page(u4 next_page)			{ header[BM_HEAD_NEXT_PAGE] = next_page; }
	void written_words(u4 written_words)	{ header[BM_HEAD_WRITTEN_WORDS] = written_words; }
	void cw_offset(u4 cw_offset)			{ header[BM_HEAD_CW_OFFSET] = cw_offset; }

	void last_page(u4 last_page)			{ header[BM_HEAD_LAST_PAGE] = last_page; }
	void last_fill_page(u4 last_fill_page)	{ header[BM_HEAD_LAST_FILL_PAGE] = last_fill_page; }
	void last_fill_pos(u4 last_fill_pos)	{ header[BM_HEAD_LAST_FILL_POS] = last_fill_pos; }

	void length(u8 length) {
		header[BM_HEAD_LENGTH_0] = (length      ) & 0xFFFFFFFF;
		header[BM_HEAD_LENGTH_1] = (length >> 32) & 0xFFFFFFFF;
	}

	void*				page;
	u4*					header;
	wah::word_t*		data;

	friend class bitmap;
	friend class biterator;
};

class bitmap
{
private: // fields
	mmf&  file;
	u4    first_page_num;
	u4    current_page_num;
	u4    written_words;
	u4    cw_offset;
	u8    length;

	void* first_page_ptr;
	void* current_page_ptr;

	bitmap_page first_page;
	bitmap_page current_page;

	wah::word_t* current_word;
	wah::word_t* last_fill_word;

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
	void new_fill(bool v);
	void load_fill();

	void init_page(u4 page);
	void load_page(u4 page);
	void update_headers();

	friend class biterator;

	static void init(mmf& file, u4 page, u4 last_fill_page, u4 last_page);
};


#endif