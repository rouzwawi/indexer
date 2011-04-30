#include "headers/fs.hpp"

#include "headers/sha1.hpp"


// do noth access fs::hash_table before calls to load_table or find_table


fs::fs(mmf& file) : file(file)
{
	// make sure the file is initialized
	u4* page_0 = (u4*) file.get_page(0);
	assert(page_0[0] == FS_MAGIC_WORD);
}

fs::~fs()
{
}

u4 fs::get_file_page(const char* filename)
{
	u4 sha1[5];
	sha1::digest(filename, sha1);

	int index = find_table(sha1, false);
	if (index == -1) return 0;
	if (!(hash_table[index].flags & FS_HE_FILLED)) return 0;
	return hash_table[index].file_page;
}

u4 fs::create_file(const char* filename)
{
	u4 sha1[5];
	sha1::digest(filename, sha1);

	// this will effectively delete any old file with this name
	// by rewriting it's page address
	int index = find_table(sha1, true);
	u4 file_page = file.allocate_page();
	hash_table[index].file_page = file_page;
	hash_table[index].flags |= FS_HE_FILLED;
	sha1::copy(hash_table[index].sha1, sha1);

	return file_page;
}

bool fs::has_file(const char* filename)
{
	// all files will have a page address != 0, only origin table will have page address == 0
	return get_file_page(filename);
}

void fs::load_table(u4 page)
{
	char* page_ptr = (char*) file.get_page(page);
	hash_table = (hash_entry*) (page_ptr + FS_TABLE_OFFSET);
	loaded_table = page;
}

int fs::find_table(u4(&sha1)[5], bool allocate)
{
	// load origin table at page 0
	load_table(0);
	int jumps = 0;
	int index = table_index(sha1, jumps);
	hash_entry* hash_entry = hash_table + index;

	// search the hash tables
	while ((hash_entry->flags & FS_HE_FILLED) && !sha1::same(sha1, hash_entry->sha1)) {
		if (!hash_entry->next_table) {
			if (!allocate) return -1; // do not allocate new tables, just return -1 (not found)
			u4 next_table_page = file.allocate_page();
			hash_entry->next_table = next_table_page;
			init_table(file.get_page(next_table_page), loaded_table);
			file.flush(next_table_page);
		}
		load_table(hash_entry->next_table);
		jumps++;
		index = table_index(sha1, jumps);
		hash_entry = hash_table + index;
	}

	// loaded table now either contains sha1 or empty slot @ index
	return index;
}

void fs::init_table(void* page_ptr, u4 parent)
{
	u4* ptr = (u4*) page_ptr;

	// make sure page is empty
	assert(ptr[0] == 0);

	// write
	ptr[0] = FS_MAGIC_WORD;
	ptr[FS_HEAD_PARENT] = parent;
}

void fs::init(mmf& file)
{
	// make sure file is empty
	assert(file.allocated_pages() == 0);

	// allocate the 0th page for origin table
	u4 page_0 = file.allocate_page();
	assert(page_0 == 0);

	void* page_ptr = file.get_page(page_0);
	init_table(page_ptr, -1);
	file.flush(page_0);

	std::cerr << "initialized filesystem" << std::endl;
}
