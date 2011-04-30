#include "headers/bitmap.hpp"

#include <algorithm>


bitmap::bitmap(mmf& file, u4 page) : file(file)
{
	first_page = page;
	first_page_ptr = file.get_page(first_page);

	first_page_header = (u4*) first_page_ptr;
	assert(first_page_header[0] == BM_MAGIC_WORD);

	// find last page and use as current
	load_page(first_page_header[BM_HEAD_LAST_PAGE]);
	assert(current_page_header[BM_HEAD_NEXT_PAGE] == 0);
}

bitmap::~bitmap()
{
	close();
}

void bitmap::append(u8* buffer, int bits)
{
	u4 lo_len = (BM_DATA_BITS - cw_offset);
	u4 hi_len = cw_offset;
	wah::word_t lo_mask = ((1ULL << lo_len) - 1);
	wah::word_t hi_mask = ((1ULL << hi_len) - 1) << lo_len;

	u4 words = bits / BM_DATA_BITS;
	u4 remain = bits % BM_DATA_BITS;

	// do we have a word split?
	if (hi_len) {
		std::cout << "two phase" << std::endl;
		register wah::word_t cw = *current_word;
		// two-phase move of low and high parts of data
		for (int i=0; i<words; i++) {
			u8 data = buffer[i];
		
			// phase 1 move low data to high word pos
			cw |= (data & lo_mask) << hi_len;

			// current word is now full
			full_word(cw);
			cw = 0;

			if (written_words == BM_DATA_WORDS) { // page is full
				full_page();
			}

			// phase 2 move high data to low word pos
			cw |= (data & hi_mask) >> lo_len;
		}
		*current_word = cw;
	} else {
		std::cout << "single phase" << std::endl;
		// single-phase move of full words
		for (int i=0; i<words; i++) {
			full_word(buffer[i]);

			if (written_words == BM_DATA_WORDS) { // page is full
				full_page();
			}
		}
	}
	
	// cw_ffset is not changed
	
	// handle remain bits
	if (remain) {
		u8 data = buffer[words];
		
		lo_len = std::min(remain, (BM_DATA_BITS - cw_offset));
		hi_len = remain - lo_len;
		lo_mask = ((1ULL << lo_len) - 1);
		hi_mask = ((1ULL << hi_len) - 1) << lo_len;
		
		*current_word |= (data & lo_mask) << cw_offset;
		cw_offset += lo_len;
		cw_offset %= BM_DATA_BITS;
		
		if (!cw_offset) { // word is full
			full_word(*current_word);
			
			if (written_words == BM_DATA_WORDS) { // page is full
				full_page();
			}
		}
		
		if (hi_len) { // bits for next word
			*current_word |= (data & hi_mask) >> lo_len;
			cw_offset += hi_len;
		}
	}
	
	update_headers();
}

void bitmap::fill(bool value, int bits)
{
	
}

void bitmap::close()
{
	file.flush(first_page);
}

inline void bitmap::full_word(wah::word_t& w)
{
	if (wah::allones(w) || wah::allzero(w)) { // compress
		std::cout << "compress" << std::endl;
		if (written_words && wah::samefill(*(current_word - 1), w)) {
			std::cout << "-inc" << std::endl;
			(*(current_word - 1))++;
		} else {
			std::cout << "-new" << std::endl;
			*current_word = (wah::allones(w) ? wah::FILL_1 : wah::FILL_0) + 1;
			current_word++;
			written_words++;
		}
	} else { // literal
		std::cout << "literal" << std::endl;
		*current_word = w;
		current_word++;
		written_words++;
	}
}

inline void bitmap::full_page()
{
	
}

void bitmap::load_page(u4 page) 
{
	current_page = page;
	current_page_ptr = file.get_page(current_page);

	// read header
	current_page_header = (u4*) current_page_ptr;
	assert(current_page_header[0] == BM_MAGIC_WORD);
	written_words = current_page_header[BM_HEAD_WRITTEN_WORDS];
	cw_offset = current_page_header[BM_HEAD_CW_OFFSET];

	// point current word pointer to active word on page
	char* data_0 = ((char*) current_page_ptr) + BM_DATA_OFFSET;
	current_word = (wah::word_t*) data_0 + written_words;
}

void bitmap::update_headers()
{
	current_page_header[BM_HEAD_WRITTEN_WORDS] = written_words;
	current_page_header[BM_HEAD_CW_OFFSET] = cw_offset;
}

void bitmap::init(mmf& file, u4 page)
{
	u4* ptr = (u4*) file.get_page(page);

	// make sure page is empty
	assert(ptr[0] == 0);

	// write header
	ptr[0] = BM_MAGIC_WORD;
	ptr[BM_HEAD_LAST_PAGE] = page;
	ptr[BM_HEAD_CW_OFFSET] = 0;
}