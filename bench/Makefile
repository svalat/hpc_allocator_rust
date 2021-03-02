# basic system setpu
CXX=g++
CFLAGS=-g -Wall
CXXFLAGS=$(CFLAGS) -O2 --std=c++11

# default target
all: bench

# build
bench: PerfTracker.cpp ProgressBar.cpp Config.cpp main.cpp
	$(CXX) $(CXXFLAGS) $^ -o $@

# header deps
Config.cpp: Config.hpp
PerfTracker.cpp: PerfTracker.hpp
ProgressBar.cpp: ProgressBar.hpp
main.cpp: PerfTracker.hpp ProgressBar.hpp Config.hpp

# clean
clean:
	rm -f main-rand-cycle-rustalloc main-rand-cycle bench

# phony targets
.PHONY: clean
