/****************************************************/
#ifndef PERF_TRACKER_HPP
#define PERF_TRACKER_HPP

/****************************************************/
#include <ostream>
#include <cstdlib>
#include <ctime>
#include <vector>
#include <cstdint>
#include <map>
#include <mutex>
#include "from-fftw/cycle.h"

/****************************************************/
namespace allocbench
{

/****************************************************/
enum EventType
{
	EVENT_MALLOC,
	EVENT_FREE,
	EVENT_MEMSET,
};

/****************************************************/
struct PerfEvent
{
	uint32_t cost;
	uint32_t memsetCost;
	void * ptr;
	uint32_t size;
	uint32_t type;
};

/****************************************************/
struct Perf
{
	Perf(void);
	void push(ticks cost);
	ticks min;
	ticks max;
	ticks sum;
	size_t cnt;
	std::vector<uint32_t> all;
};

/****************************************************/
class PerfResults
{
	public:
		PerfResults(void);
		double push(PerfEvent * events, size_t count);
		void print(void);
	private:
		std::map<size_t, Perf> perfMapMalloc;
		std::map<size_t, Perf> perfMapFree;
		std::map<size_t, Perf> perfMapMemset;
		std::map<size_t, Perf> perfMapFullOps;
		std::map<void*, PerfEvent*> sizeMap;
};

/****************************************************/
class PerfTracker
{
	public:
		PerfTracker(size_t maxNbOps, bool enabled);
		~PerfTracker(void);
		void start(void);
		void stop(void);
		size_t getId(void);
		size_t flush(void);
		void onMalloc(void * ptr, size_t size, ticks cost, ticks memsetCost);
		void onFree(void * ptr, ticks cost);
		void onMemset(void * ptr, size_t size, ticks cost);
		void printResults(void);
	private:
		size_t maxNbOps;
		size_t cursor;
		size_t memSize;
		PerfEvent * events;
		struct timespec tstart;
		struct timespec tstop;
		double tpause;
		bool enabled;
		PerfResults results;
		std::mutex mutex;
};

/****************************************************/
std::ostream & operator<<(std::ostream & out, Perf & perf);

}

#endif // PERF_TRACKER_HPP
