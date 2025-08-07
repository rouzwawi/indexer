#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/lexical_cast.hpp>

#include <boost/date_time/posix_time/posix_time.hpp>

#include <map>
#include <list>
#include <iterator>

#include "headers/typedefs.hpp"

#include "headers/sha1.hpp"
#include "headers/wah.hpp"

#include "headers/fs.hpp"
#include "headers/mmf.hpp"
#include "headers/bitmap.hpp"
#include "headers/biterator.hpp"

#define LINES 64


using namespace std;
using namespace boost::interprocess;
using boost::lexical_cast;

namespace test {

template <class T>
struct b {
   b(){} ~b(){}
   inline string& p() { return static_cast<T*>(this)->v; }
};

#define perm(x,y,z) template <> struct n <x,y,z> : b< n<x,y,x> >
#define vexp(x,y,z) string v; n():v("n " #x #y #z){}

template <op T0, op T1, op T2> struct n : b< n<T0 , T1 , T2 > > {string v; n():v("n ___"){}};

template <       op T1, op T2> struct n <X, T1, T2> : b< n<X, T1, T2> > {string v; n():v("n X__"){}};
template <       op T1, op T2> struct n <A, T1, T2> : b< n<A, T1, T2> > {string v; n():v("n A__"){}};
template <       op T1, op T2> struct n <O, T1, T2> : b< n<O, T1, T2> > {string v; n():v("n O__"){}};

// two operator boperator cases  here

perm(X,O,D) { vexp(X,O,D) };
perm(X,A,D) { vexp(X,A,D) };
perm(X,X,D) { vexp(X,X,D) };
perm(X,N,D) { vexp(X,N,D) };
perm(A,O,D) { vexp(A,O,D) };
perm(A,A,D) { vexp(A,A,D) };
perm(A,X,D) { vexp(A,X,D) };
perm(A,N,D) { vexp(A,N,D) };
perm(O,O,D) { vexp(O,O,D) };
perm(O,A,D) { vexp(O,A,D) };
perm(O,X,D) { vexp(O,X,D) };
perm(O,N,D) { vexp(O,N,D) };

perm(O,D,D) { vexp(O,D,D) };
perm(A,D,D) { vexp(A,D,D) };
perm(X,D,D) { vexp(X,D,D) };

perm(N,O,E) { vexp(N,O,E) };
perm(N,A,E) { vexp(N,A,E) };
perm(N,X,E) { vexp(N,X,E) };
perm(N,N,E) { vexp(N,N,E) };
perm(N,D,E) { vexp(N,D,E) };


}
using namespace test;

void wah()
{
   std::cout << wah::WORD_LENGTH << std::endl;
   std::cout << std::hex << wah::DATA_BITS << std::endl;
   std::cout << std::hex << wah::FILL_FLAG << std::endl;
   std::cout << std::hex << wah::FILL_VAL  << std::endl;
   std::cout << std::endl;

   std::cout << wah::isfill(0x7FFFFFFFFFFFFFFFLLU) << std::endl;
   std::cout << wah::isfill(0x8FFFFFFFFFFFFFFFLLU) << std::endl;
   std::cout << wah::fillval(0x8FFFFFFFFFFFFFFFLLU) << std::endl;
   std::cout << wah::fillval(0xCFFFFFFFFFFFFFFFLLU) << std::endl;
}

void test_callback(u8 word, u8 mask) { cout << word << endl; }

int test_itop()
{
}

int test_addr_calcs()
{
   assert(0x00001000 == mmf::page_size());
   assert(0x00800000 == mmf::region_size());

   assert(0x00800000 == mmf::file_size(0));
   assert(0x01000000 == mmf::file_size(1));
   assert(0x02000000 == mmf::file_size(2));
   assert(0x04000000 == mmf::file_size(3));
   assert(0x08000000 == mmf::file_size(4));
   assert(0x10000000 == mmf::file_size(5));
   assert(0x20000000 == mmf::file_size(6));
   assert(0x40000000 == mmf::file_size(7));
   assert(0x40000000 == mmf::file_size(8));

   assert(  1 == mmf::file_regions(0));
   assert(  2 == mmf::file_regions(1));
   assert(  4 == mmf::file_regions(2));
   assert(  8 == mmf::file_regions(3));
   assert( 16 == mmf::file_regions(4));
   assert( 32 == mmf::file_regions(5));
   assert( 64 == mmf::file_regions(6));
   assert(128 == mmf::file_regions(7));
   assert(128 == mmf::file_regions(8));

   assert(  0 == mmf::regions_up_to(0));
   assert(  1 == mmf::regions_up_to(1));
   assert(  3 == mmf::regions_up_to(2));
   assert(  7 == mmf::regions_up_to(3));
   assert( 15 == mmf::regions_up_to(4));
   assert( 31 == mmf::regions_up_to(5));
   assert( 63 == mmf::regions_up_to(6));
   assert(127 == mmf::regions_up_to(7));
   assert(255 == mmf::regions_up_to(8));

   assert( 0 == mmf::file_addr(  0));
   assert( 1 == mmf::file_addr(  1));
   assert( 1 == mmf::file_addr(  2));
   assert( 2 == mmf::file_addr(  3));
   assert( 2 == mmf::file_addr(  6));
   assert( 3 == mmf::file_addr(  7));
   assert( 3 == mmf::file_addr( 14));
   assert( 4 == mmf::file_addr( 15));
   assert( 4 == mmf::file_addr( 30));
   assert( 5 == mmf::file_addr( 31));
   assert( 5 == mmf::file_addr( 62));
   assert( 6 == mmf::file_addr( 63));
   assert( 6 == mmf::file_addr(126));
   assert( 7 == mmf::file_addr(127));
   assert( 7 == mmf::file_addr(254));
   assert( 8 == mmf::file_addr(255));
   assert( 8 == mmf::file_addr(382));
   assert( 9 == mmf::file_addr(383));

   return 0;
}

u4 get_or_alloc(std::string& fileName, fs& fileSystem)
{
   u4 page;
   if(fileSystem.has_file(fileName.c_str())) {
      cout << "file '" << fileName << "' exists @ page " << hex << (page = fileSystem.get_file_page(fileName.c_str())) << endl;
   } else {
      page = fileSystem.create_file(fileName.c_str());
      bitmap::init(fileSystem.get_file(), page);
      cout << "allocated '" << fileName << "' @ page " << hex << page << endl;
   }
   return page;
}

void test_inverses(fs& fileSystem)
{
   std::string f0("inv-file0");
   std::string f1("inv-file1");
   u4 page_f0 = get_or_alloc(f0, fileSystem);
   u4 page_f1 = get_or_alloc(f1, fileSystem);

   bitmap bm0(fileSystem.get_file(), page_f0);
   bitmap bm1(fileSystem.get_file(), page_f1);
   u8 bits[512];
   u8 bivs[512];
   int dens = 997;

   for (int j=0; j<512; j++) {
      bits[j] = U8(0);
      for (int b=0; b<wah::WORD_LENGTH; b++)
         if (rand() % 1000 < dens) bits[j] |= U8(1) << b;
      bivs[j] = ~bits[j]; // inv
   }

   boost::posix_time::ptime ts(boost::posix_time::microsec_clock::universal_time());
   for (int i=0; i<4*1024; i++) {
      bm0.append(bits, 512*wah::WORD_LENGTH);
      bm1.append(bivs, 512*wah::WORD_LENGTH);
   }
   boost::posix_time::ptime te(boost::posix_time::microsec_clock::universal_time());
   cout << "wrote " << dec << 512*4*1024*wah::WORD_LENGTH << " bits in " << (te-ts).total_microseconds() << " us" << endl;
   cout << "rate " << 512*4*1024*wah::WORD_LENGTH / (te-ts).total_microseconds() << " bits/us" << endl;
   cout << endl;

   biterator it0(fileSystem.get_file(), page_f0);
   biterator it1(fileSystem.get_file(), page_f1);
   boperator bop(O, it0, it1);

   ts = boost::posix_time::ptime(boost::posix_time::microsec_clock::universal_time());
   while (bop.has_next()) {
      u8 nxt = bop.next();
      assert(nxt == wah::DATA_BITS);
   }
   te = boost::posix_time::ptime(boost::posix_time::microsec_clock::universal_time());
   cout << "iterated " << dec << bop.bits() << " bits in " << (te-ts).total_microseconds() << " us" << endl;
   cout << "rate " << bop.bits() / (te-ts).total_microseconds() << " bits/us" << endl;
}

int main(int argc, const char* argv[])
{
   // std::string uuu; cin >> uuu;

   n<X, A, D> n_XAD;
   n<A, O, X> n_AOX;
   n<O, X, D> n_OXD;
   n<N, X, E> n_NXT;

   cout << n_XAD.p() << endl;
   cout << n_AOX.p() << endl;
   cout << n_OXD.p() << endl;
   cout << n_NXT.p() << endl;

   // b<deriv<AND>>* bp = &fooand;
   // cout << bp << endl;

   test_addr_calcs();
   test_itop();

   mmf f(argv[1]);

   if (f.allocated_pages() == 0)
      fs::init(f);

   fs fileSystem(f);

   //u4 index_page_0 = lexical_cast<u4>(argv[2]);
   //bitmap bitmap(data_file, index_page_0);

   std::string x;
   std::string y;
   while (cin >> x) {
      if (x == "q") exit(0);

      if (x == "a") { // append
         cin >> y;
         u4 page;
         if(fileSystem.has_file(y.c_str())) {
            cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
         } else {
            page = fileSystem.create_file(y.c_str());
            bitmap::init(f, page);
            cout << "allocated '" << y << "' @ page " << hex << page << endl;
         }
         bitmap bm(f, page);
         u8 bits[] = {U8(0x7FFFFFFFFFFFFFFF), U8(0x7FFFFFFFFFFFFFFF), U8(0xAAA), U8(0xCCC), U8(0xFFF)};
         for (int v=0;v<1;v++)
            bm.append(bits, 263);
         continue;
      }

      if (x == "b") {
         test_inverses(fileSystem);
         continue;
      }

      if (x == "d") { // density fill
         cin >> y;
         u4 dens, n;
         stringstream ss;
         cin >> x;
         ss << dec << x; ss >> n; ss.clear();
         cin >> x;
         ss << dec << x; ss >> dens;
         u4 page;
         if(fileSystem.has_file(y.c_str())) {
            cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
         } else {
            page = fileSystem.create_file(y.c_str());
            bitmap::init(f, page);
            cout << "allocated '" << y << "' @ page " << hex << page << endl;
         }
         bitmap bm(f, page);
         for (int i=0; i<n; i++) {
            u8 bits[] = {U8(0)};
            for (int b=0; b<wah::WORD_LENGTH; b++)
               if (rand() % 100 < dens) bits[0] |= U8(1) << b;
            cout << "appending " << hex << bits[0] << endl;
            bm.append(bits, wah::WORD_LENGTH);
         }
         continue;
      }

      if (x == "i") { // iterate
         cin >> y;
         u4 page;
         if(fileSystem.has_file(y.c_str())) {
            cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
         } else {
            cout << "file '" << y << "' not found...abort iteration" << endl;
            continue;
         }
         biterator it(f, page);
         while (it.has_next()) {
            cout << hex << it.next();
         }
         cout << it.last_word_mask() << endl;
         cout << "EOF" << endl;

         continue;
      }

      using namespace tmplt;
      if (x == "o") { // operate
         cin >> y;
         u4 page;
         if(fileSystem.has_file(y.c_str())) {
            cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
         } else {
            cout << "file '" << y << "' not found...abort iteration" << endl;
            continue;
         }
         biterator it0(f, page);
         biterator it1(f, page);

         boperator bop(O, it0, it1);
         cout << "children" << endl;
         const list<noderator*>& c = bop.c();
         for(list<noderator*>::const_iterator ct = c.begin(); ct != c.end(); ++ct)
            cout << *ct << endl;
         cout << "is op case 0 : " << op_case<bol, var, any>::is(true, &bop, &bop) << endl;
         cout << "is op case 1 : " << op_case<any, zer, var>::is(&bop, &bop) << endl;
         cout << "skip some" << endl;
         bop.skip_words(2);
         cout << "is op case 1 : " << op_case<any, zer, var>::is(&bop, &bop) << endl;

         continue;
      }

      if (x == "s") { // skip
         cin >> y;
         u4 page;
         if(fileSystem.has_file(y.c_str())) {
            cout << "file '" << y << "' exists @ page " << hex << (page = fileSystem.get_file_page(y.c_str())) << endl;
         } else {
            cout << "file '" << y << "' not found...abort iteration" << endl;
            continue;
         }
         biterator it(f, page);

         it.skip_words(600);
         continue;
      }

      if (x == "w") { // write
         cin >> y;

         if(fileSystem.has_file(y.c_str())) {
            u4 file_page = fileSystem.get_file_page(y.c_str());
            int* ptr = (int*) f.get_page(file_page);
            int* p;
            for (int x=0;x<PAGE_SIZE/sizeof(int);x++) {
               p = ptr + x;
               for (int y=0;y<(1<<20)-1;y++) {
                  (*p)++;
               }
            }
            cout << "wrote to file '" << y << "' @ page " << hex << file_page << endl;
         } else {
            cout << "file '" << y << "' does not exists" << endl;
         }
         continue;
      }

      u4 page;
      stringstream ss;
      ss << hex << x;
      ss >> page;
      cout << "page " << dec << page << endl;
      char* p = (char*) f.get_page(page);
      for (int i=0; i<LINES; i++) {
         for (int j=0; j<64; j++) {
            int v = ((int)p[i*64 + j] & 0xFF);
            if (j % 4 == 0) cout << " ";
            cout << hex << (v<0x10?"0":"") << v;
         }
         cout << endl;
      }
   }

   return 0;
}
