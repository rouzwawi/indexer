TARGET  := indexer.out
SRCS    := test.cpp mmf.cpp bitmap.cpp
OBJS    := ${SRCS:.cpp=.o} 
DEPS    := ${SRCS:.cpp=.dep} 
XDEPS   := $(wildcard ${DEPS}) 

CCFLAGS = 
LDFLAGS = 
LIBS    = 

COMPILER = g++

.PHONY: all clean distclean 
all:: ${TARGET} 

ifneq (${XDEPS},) 
include ${XDEPS} 
endif 

${TARGET}: ${OBJS} 
	${COMPILER} ${LDFLAGS} ${LIBS} -o $@ $^ ${LIBS} 

${OBJS}: %.o: %.cpp %.dep 
	${COMPILER} ${CCFLAGS} ${LIBS} -o $@ -c $< 

${DEPS}: %.dep: %.cpp Makefile 
	${COMPILER} ${CCFLAGS} ${LIBS} -MM $< > $@ 

clean:: 
	-rm -f *~ *.o *.dep ${TARGET} 

distclean:: clean
