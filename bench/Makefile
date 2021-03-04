# basic system setpu
CXX=g++
CFLAGS=-g -Wall
CXXFLAGS=$(CFLAGS) -O1 --std=c++11
LDFLAGS=
LDFLAGS=-Lallocs/ -lhpc_allocator_rust -Wl,-rpath,$$PWD/allocs

# default target
all: bench libtracer.so

# tracer to dump alloc traces.
libtracer.so: tracer.cpp from-mpc-allocator-cpp/PosixAllocatorStd.cpp from-mpc-allocator-cpp/PosixAllocatorFileTrace.cpp
	$(CXX) $(CXXFLAGS) -shared -fPIC -o $@ $^

# build
bench: PerfTracker.cpp ProgressBar.cpp Config.cpp bench.cpp
	$(CXX) $(CXXFLAGS) $(LDFLAGS) -o $@ $^

# header deps
Config.cpp: Config.hpp
PerfTracker.cpp: PerfTracker.hpp
ProgressBar.cpp: ProgressBar.hpp
bench.cpp: PerfTracker.hpp ProgressBar.hpp Config.hpp
tracer.cpp: from-mpc-allocator-cpp/PosixAllocatorStd.h from-mpc-allocator-cpp/PosixAllocatorFileTrace.h
from-mpc-allocator-cpp/PosixAllocatorStd.cpp: from-mpc-allocator-cpp/PosixAllocatorStd.h
from-mpc-allocator-cpp/PosixAllocatorFileTrace.cpp: from-mpc-allocator-cpp/PosixAllocatorFileTrace.h from-mpc-allocator-cpp/PosixAllocatorStd.h

# clean
clean:
	rm -f bench
	rm -f libtracer.so

# phony targets
.PHONY: clean
