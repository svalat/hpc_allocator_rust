#include <vector>
#include <string>
#include <iostream>
#include <random>
#include <cstdlib>
#include <cassert>
#include <cstring>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>
#include "Config.hpp"
#include "ProgressBar.hpp"
#include "PerfTracker.hpp"
#include "from-mpc-allocator-cpp/AllocTraceStruct.h"

/****************************************************/
using namespace MPCAllocator;
using namespace allocbench;
using namespace std;

/****************************************************/
#define MEASURE(code) do { \
		before = getticks();\
		code;\
		after = getticks();\
		cost = after - before; \
	} while(0)

/****************************************************/
class SizeGenerator
{
	public:
		SizeGenerator(const Config & config);
		~SizeGenerator(void);
		size_t getSize(void);
	private:
		enum BenchMode benchMode;
		size_t fixed;
		size_t randMin;
		size_t randMax;
		size_t randStep;
		size_t randCntSteps;
		size_t listCount;
		random_device randDevice;
		mt19937 randGenerator;
		uniform_int_distribution<size_t> * randDistr;
		std::vector<size_t> sizeList;
};

/****************************************************/
SizeGenerator::SizeGenerator(const Config & config)
              :randGenerator(0/*randDevice()*/)
{
	//store values
	this->benchMode = config.bench;
	this->randDistr = nullptr;
	this->fixed = 0;
	this->randMin = 0;
	this->randMax = 0;
	this->randStep = 0;
	this->randCntSteps = 0;
	this->listCount = 0;
	
	//extract
	switch(this->benchMode) {
		case BENCH_FIXED:
			this->fixed = config.sizes[0];
			break;
		case BENCH_LIST:
			this->sizeList = config.sizes;
			this->listCount = config.sizes.size();
			this->randDistr = new uniform_int_distribution<size_t>(0, this->listCount);
			break;
		case BENCH_RAND:
			this->randMin = config.sizes[0];
			this->randStep = config.sizes[1];
			this->randMax = config.sizes[2];
			this->randCntSteps = (this->randMax - this->randMin) / this->randStep;
			this->randDistr = new uniform_int_distribution<size_t>(0, this->randCntSteps);
			break;
		case BENCH_TRACE:
			assert(false);
			break;
	}
}

/****************************************************/
SizeGenerator::~SizeGenerator(void)
{
	if (this->randDistr != nullptr)
		delete this->randDistr;
}

/****************************************************/
size_t SizeGenerator::getSize(void)
{
	//var
	size_t size = 0;
	size_t id = 0;
	//switch
	switch(this->benchMode) {
		case BENCH_FIXED:
			size = this->fixed;
			break;
		case BENCH_LIST:
			id = (*this->randDistr)(this->randGenerator);
			size = this->sizeList[id];
			break;
		case BENCH_RAND:
			id = (*this->randDistr)(this->randGenerator);
			size = this->randMin + this->randStep * id;
			break;
		case BENCH_TRACE:
			assert(false);
			break;
	}

	//ret
	return size;
}

/****************************************************/
void benchLinear(PerfTracker & perf, const Config & config)
{
	//vars
	ticks before, after, cost;
	SizeGenerator sizeGenerator(config);
	ProgressBar progressBar(60, config.iterations, config.progress);

	//allocate ptr storage
	void ** ptr = new void*[config.keep];
	memset(ptr, 0, sizeof(void*) * config.keep);

	//start bench
	perf.start();

	//loop
	for (size_t i = 0 ; i < config.iterations ; i++) {
		//get storage id
		size_t id = i % config.keep;

		//progress
		progressBar.progress(i);

		//free
		if (ptr[id] != nullptr) {
			MEASURE(free(ptr[id]));
			perf.onFree(ptr[id], cost);
			ptr[id] = nullptr;
		}

		//get alloc size
		size_t size = sizeGenerator.getSize();

		//alloc
		MEASURE(ptr[id] = malloc(size));
		ticks mallocCost = cost;

		//memset
		ticks memsetCost = 0;
		if (config.memset) {
			MEASURE(memset(ptr[id], 0, size));
			memsetCost = cost;
			perf.onMemset(ptr[id], size, cost);
		}

		//push
		perf.onMalloc(ptr[id], size, mallocCost, memsetCost);
	}

	//clear all
	for (size_t id = 0 ; id < config.keep ; id++) {
		//free
		if (ptr[id] != nullptr) {
			MEASURE(free(ptr[id]));
			perf.onFree(ptr[id], cost);
			ptr[id] = nullptr;
		}
	}

	//stop bench
	perf.stop();
}

/****************************************************/
void benchFull(PerfTracker & perf, const Config & config)
{
	//vars
	ticks before, after, cost;
	SizeGenerator sizeGenerator(config);
	ProgressBar progressBar(60, config.iterations, config.progress);

	//allocate ptr storage
	void ** ptr = new void*[config.keep];
	memset(ptr, 0, sizeof(void*) * config.keep);

	//start bench
	perf.start();

	//loop
	for (size_t i = 0 ; i < config.iterations ; i++) {
		//get storage id
		size_t id = i % config.keep;

		//progress
		progressBar.progress(i);

		//clear all
		if (id == 0 && i != 0) {
			//clear all
			for (size_t j = 0 ; j < config.keep ; j++) {
				//free
				if (ptr[j] != nullptr) {
					MEASURE(free(ptr[j]));
					perf.onFree(ptr[j], cost);
					ptr[j] = nullptr;
				}
			}
		}

		//get alloc size
		size_t size = sizeGenerator.getSize();

		//alloc
		MEASURE(ptr[id] = malloc(size));
		ticks mallocCost = cost;

		//memset
		ticks memsetCost = 0;
		if (config.memset) {
			MEASURE(memset(ptr[id], 0, size));
			memsetCost = cost;
			perf.onMemset(ptr[id], size, cost);
		}

		//push
		perf.onMalloc(ptr[id], size, mallocCost, memsetCost);
	}

	//clear all
	for (size_t id = 0 ; id < config.keep ; id++) {
		//free
		if (ptr[id] != nullptr) {
			MEASURE(free(ptr[id]));
			perf.onFree(ptr[id], cost);
			ptr[id] = nullptr;
		}
	}

	//stop bench
	perf.stop();
}

/****************************************************/
void benchRand(PerfTracker & perf, const Config & config)
{
	//vars
	ticks before, after, cost;
	SizeGenerator sizeGenerator(config);
	ProgressBar progressBar(60, config.iterations, config.progress);
	random_device randomDevice;
	mt19937_64 randomGenerator(0/*randomDevice()*/);
	uniform_int_distribution<size_t> randomDistr(0, config.keep - 1);

	//allocate ptr storage
	void ** ptr = new void*[config.keep];
	memset(ptr, 0, sizeof(void*) * config.keep);

	//start bench
	perf.start();

	//loop
	for (size_t i = 0 ; i < config.iterations ; i++) {
		//get storage id
		size_t id = randomDistr(randomGenerator);
		assert(id < config.keep);

		//progress
		progressBar.progress(i);

		//free
		if (ptr[id] != nullptr) {
			MEASURE(free(ptr[id]));
			perf.onFree(ptr[id], cost);
			ptr[id] = nullptr;
		}

		//get alloc size
		size_t size = sizeGenerator.getSize();

		//alloc
		MEASURE(ptr[id] = malloc(size));
		ticks mallocCost = cost;

		//memset
		ticks memsetCost = 0;
		if (config.memset) {
			MEASURE(memset(ptr[id], 0, size));
			memsetCost = cost;
			perf.onMemset(ptr[id], size, cost);
		}

		//push
		perf.onMalloc(ptr[id], size, mallocCost, memsetCost);
	}

	//clear all
	for (size_t id = 0 ; id < config.keep ; id++) {
		//free
		if (ptr[id] != nullptr) {
			MEASURE(free(ptr[id]));
			perf.onFree(ptr[id], cost);
			ptr[id] = nullptr;
		}
	}

	//stop bench
	perf.stop();
}

/****************************************************/
long getFileSize(std::string filename)
{
    struct stat stat_buf;
    int rc = stat(filename.c_str(), &stat_buf);
    return rc == 0 ? stat_buf.st_size : -1;
}

/****************************************************/
void benchTrace(PerfTracker & perf, const Config & config)
{
	//vars
	ticks before, after, cost;
	ProgressBar progressBar(60, config.iterations, config.progress);

	//get file size & rount to multiple of 4K
	size_t traceSize = getFileSize(config.trace);
	size_t traceEntries = traceSize / sizeof(TraceEntry);
	assert(traceSize >= 0);
	if (traceSize % 4096 != 0)
		traceSize += 4096 - traceSize % traceSize;

	//mmap to memory
	int fd = open(config.trace.c_str(), 0);
	assert(fd > 0);
	void * tracePtr = mmap(NULL, traceSize, PROT_READ|PROT_WRITE, MAP_FILE|MAP_PRIVATE, fd, 0);
	assert(tracePtr != MAP_FAILED);
	TraceEntry * trace = static_cast<TraceEntry*>(tracePtr);

	//start bench
	perf.start();

	//loop
	for (size_t i = 0 ; i < config.iterations ; i++) {
		//get storage id
		size_t id = i % traceEntries;

		//progress
		progressBar.progress(i);

		//treat entry
		TraceEntry & entry = trace[id];

		//case
		if (entry.type == TRACE_MALLOC) {
			//alloc
			void * ptr = NULL;
			size_t size = entry.size;
			MEASURE(ptr = malloc(size));
			ticks mallocCost = cost;

			//memset
			ticks memsetCost = 0;
			if (config.memset) {
				MEASURE(memset(ptr, 0, size));
				memsetCost = cost;
				perf.onMemset(ptr, size, cost);
			}

			//round
			size_t power = log2(size);
			size = 1<<power;

			//push
			perf.onMalloc(ptr, size, mallocCost, memsetCost);

			//store
			entry.ptrInfo.ptr = ptr;
		} else if (entry.type == TRACE_FREE) {
			TraceEntry & allocEntry = trace[entry.ptrInfo.ptrIndex];
			if (allocEntry.type == TRACE_MALLOC) {
				void * ptr = allocEntry.ptrInfo.ptr;
				MEASURE(free(ptr));
				perf.onFree(ptr, cost);
				allocEntry.ptrInfo.ptr = nullptr;
			}
		}
	}

	//stop bench
	perf.stop();
}

/****************************************************/
int main(int argc, char ** argv)
{
	//parse args
	Config config;
	config.parse(argc, argv);

	//print
	if (config.quiet == false)
		config.print();

	//check size
	switch (config.bench) {
		case BENCH_FIXED:
			assert(config.sizes.size() == 1);
			break;
		case BENCH_LIST:
			assert(config.sizes.size() > 0);
			break;
		case BENCH_RAND:
			assert(config.sizes.size() == 3);
			assert(config.sizes[1] <= config.sizes[2] - config.sizes[1]);
			assert(config.sizes[0] < config.sizes[2]);
			break;
		case BENCH_TRACE:
			break;
	}

	//max store
	size_t maxOpsStore = ((size_t)(config.opCache * 1024.0 * 1024.0 * 1024.0)) / sizeof(PerfEvent);
	size_t opStore = config.iterations * 3;
	if (opStore > maxOpsStore)
		opStore = maxOpsStore;

	//run
	PerfTracker perf(opStore, config.perf);
	if (config.bench == BENCH_TRACE) {
		benchTrace(perf, config);
	} else {
		switch(config.reuse) {
			case REUSE_LINEAR:
				benchLinear(perf, config);
				break;
			case REUSE_FULL:
				benchFull(perf, config);
				break;
			case REUSE_RAND:
				benchRand(perf, config);
				break;
		}
	}

	//print
	perf.printResults();

	//finish
	return EXIT_SUCCESS;
}
