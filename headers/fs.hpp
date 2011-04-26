#include "typedefs.hpp"
#include "mmf.hpp"


#ifndef FS_H
#define FS_H

#define MAGIC_WORD 0x72786469

#define TABLE_OFFSET 64
#define TABLE_ENTRIES 113

#define HE_FILLED 0x00000001


typedef struct hash_entry
{
	u4 sha1[5];
	u4 file_page;
	u4 next_table;
	u4 flags;
} hash_entry;


BOOST_STATIC_ASSERT(sizeof(hash_entry) == 32);

class fs
{

private:
	mmf& file;
	hash_entry* hash_table;

public:
	fs(mmf& file);
	~fs();

	u4 get_file_page(const char* filename);
	u4 create_file(const char* filename);
	bool has_file(const char* filename);
	
	static void init(mmf& file);

private:
	void load_table(u4 page);
	int find_table(u4(&sha1)[5], bool allocate);

	static int table_index(const u4(&sha1)[5], int jumps) { return sha1[jumps % 5] % TABLE_ENTRIES; }
	static void magic_word(void* page_ptr);
};


#endif
