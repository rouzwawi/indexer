#include <boost/uuid/sha1.hpp>


#ifndef SHA1_H
#define SHA1_H


class sha1
{

public:
	static void digest(char const* str, unsigned int (&digest)[5]) {
		boost::uuids::detail::sha1 s;
		s.process_bytes(str, strlen(str));
		s.get_digest(digest);
	}

};


#endif
