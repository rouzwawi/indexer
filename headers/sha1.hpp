#include <boost/uuid/sha1.hpp>


#ifndef SHA1_H
#define SHA1_H


class sha1
{

public:
	typedef unsigned int(&sha1_type)[5];
	
	static void digest(const char* str, sha1_type digest) {
		boost::uuids::detail::sha1 s;
		s.process_bytes(str, strlen(str));
		s.get_digest(digest);
	}

	static bool same(sha1_type digest1, sha1_type digest2) {
		for (int i=0; i<5; i++) if (digest1[i] != digest2[i]) return false;
		return true;
	}

	static void copy(sha1_type to, sha1_type from) {
		memcpy(to, from, sizeof(sha1_type));
	}

};


#endif
