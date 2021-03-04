/****************************************************/
#include <map>
#include <algorithm>
#include <iostream>
#include <cassert>
#include <cstring>
#include <sys/mman.h>
#include "PerfTracker.hpp"

/****************************************************/
using namespace std;
using namespace allocbench;

/****************************************************/
#define PAGE_SIZE 4096

/****************************************************/
static inline double timespecDiff(const struct timespec &a, const struct timespec &b) {
	struct timespec result;
	result.tv_sec  = a.tv_sec  - b.tv_sec;
	result.tv_nsec = a.tv_nsec - b.tv_nsec;
	if (result.tv_nsec < 0) {
		--result.tv_sec;
		result.tv_nsec += 1000000000L;
	}
	return (double)result.tv_sec + (double)result.tv_nsec / (double)1e9;
}

/****************************************************/
Perf::Perf(void)
{
	this->cnt = 0;
	this->min = -1;
	this->max = 0;
	this->sum = 0;
}

/****************************************************/
void Perf::push(ticks cost)
{
	this->cnt++;
	this->sum += cost;
	if (cost < this->min)
		this->min = cost;
	if (cost > this->max)
		this->max = cost;
	this->all.push_back(cost);
}

/****************************************************/
PerfTracker::PerfTracker(size_t maxNbOps, bool enabled)
{
	//setup
	this->maxNbOps = maxNbOps;
	this->cursor = 0;
	this->enabled = enabled;
	this->tpause = 0.0;

	//calc size & round to 4K
	this->memSize = maxNbOps * sizeof(PerfEvent);
	if (this->memSize % PAGE_SIZE)
		this->memSize = this->memSize + PAGE_SIZE - this->memSize % PAGE_SIZE;
	
	//allocate
	if (enabled)
	{
		cout << "# Mem size: " << this->memSize / 1024 / 1024 << " MB" << endl;
		void * ptr = mmap(NULL, this->memSize, PROT_READ|PROT_WRITE, MAP_ANON|MAP_PRIVATE, 0, 0);
		assert(ptr != MAP_FAILED);
		memset(ptr, 0, this->memSize);
		this->events = static_cast<PerfEvent*>(ptr);
	}
}

/****************************************************/
PerfTracker::~PerfTracker(void)
{
	if (enabled)
	{
		int res = munmap(this->events, this->memSize);
		assert(res == 0);
	}
}

/****************************************************/
void PerfTracker::onMalloc(void * ptr, size_t size, ticks cost, ticks memsetCost)
{
	//check
	if (!this->enabled)
		return;

	//get id
	size_t id = this->getId();

	//check
	assert(id < this->maxNbOps);

	//store
	this->events[id].type = EVENT_MALLOC;
	this->events[id].ptr = ptr;
	this->events[id].size = size;
	this->events[id].cost = cost;
	this->events[id].memsetCost = memsetCost;
}

/****************************************************/
void PerfTracker::onFree(void * ptr, ticks cost)
{
	//check
	if (!this->enabled)
		return;

	//get id
	size_t id = this->getId();

	//check
	assert(id < this->maxNbOps);

	//store
	this->events[id].type = EVENT_FREE;
	this->events[id].ptr = ptr;
	this->events[id].cost = cost;
}

/****************************************************/
void PerfTracker::onMemset(void * ptr, size_t size, ticks cost)
{
	//check
	if (!this->enabled)
		return;

	//get id
	size_t id = this->getId();

	//check
	assert(id < this->maxNbOps);

	//store
	this->events[id].type = EVENT_MEMSET;
	this->events[id].ptr = ptr;
	this->events[id].size = size;
	this->events[id].cost = cost;
}

/****************************************************/
size_t PerfTracker::getId(void)
{
	//get id
	size_t id = __sync_fetch_and_add(&this->cursor, 1);

	//check
	if (id >= this->maxNbOps)
		id = this->flush();

	return id;
}

/****************************************************/
size_t PerfTracker::flush(void)
{
	//vars
	lock_guard<std::mutex> guard(this->mutex);
	
	//check
	size_t id = __sync_fetch_and_add(&this->cursor, 1);
	if (id < this->maxNbOps)
		return id;

	//flush
	cout << "# Flush op cache" << endl;
	this->tpause += this->results.push(this->events, this->maxNbOps);
	this->cursor = 0;

	//return
	return 0;
}

/****************************************************/
PerfResults::PerfResults(void)
{

}

/****************************************************/
double PerfResults::push(PerfEvent * events, size_t count)
{
	//vars
	timespec t0, t1;

	//measure start
	clock_gettime(CLOCK_MONOTONIC, &t0);

	//loop to fill
	for (size_t i = 0 ; i < count ; i++) {
		PerfEvent & event = events[i];
		if (event.type == EVENT_MALLOC) {
			perfMapMalloc[event.size].push(event.cost);
			sizeMap[event.ptr] = &event;
		} else if (event.type == EVENT_FREE) {
			auto it = sizeMap.find(event.ptr);
			if(it != sizeMap.end()) {
				perfMapFree[it->second->size].push(event.cost);
				perfMapFullOps[it->second->size].push(event.cost + it->second->cost + it->second->memsetCost);
				sizeMap.erase(it);
			}
		} else if (event.type == EVENT_MEMSET) {
			perfMapMemset[event.size].push(event.cost);
		} else {
			assert(false);
		}
	}

	//reset 
	memset(events, 0, sizeof(PerfEvent) * count);

	//measure end
	clock_gettime(CLOCK_MONOTONIC, &t1);

	//add to pause
	return timespecDiff(t1,t0);
}


/****************************************************/
void PerfResults::print(void)
{
	//header
	cout << "#Operation\tSize\tMin\tQuartils 20%\tAverage\tQuartils 80%\tMax" << endl;

	//print
	for (auto &it : perfMapMalloc)
		cout << "MALLOC\t" << it.first << "\t" << it.second << endl;
	for (auto &it : perfMapFree)
		cout << "FREE\t" << it.first << "\t" << it.second << endl;
	for (auto &it : perfMapMemset)
		cout << "MEMSET\t" << it.first << "\t" << it.second << endl;
	for (auto &it : perfMapFullOps)
		cout << "FULLOPS\t" << it.first << "\t" << it.second << endl;
}

/****************************************************/
void PerfTracker::printResults(void)
{
	//flush
	this->results.push(events, cursor);

	//print
	this->results.print();

	//total time
	cout << "TOTAL\t" << timespecDiff(this->tstop, this->tstart) - this->tpause << endl;
}

/****************************************************/
void PerfTracker::start(void)
{
	clock_gettime(CLOCK_MONOTONIC, &this->tstart);
}

/****************************************************/
void PerfTracker::stop(void)
{
	clock_gettime(CLOCK_MONOTONIC, &this->tstop);
}

/****************************************************/
std::ostream & allocbench::operator<<(std::ostream & out, Perf & perf)
{
	std::sort(perf.all.begin(), perf.all.end());
	//min
	out << perf.min << "\t";

	//quartil 20%
	size_t size = perf.all.size();
	size_t margin = size / 5;
	out << perf.all[margin] << "\t";

	//average
	float average = (float)perf.sum / (float)perf.cnt;
	out << "\t" << average << "\t";
	
	//quartil 80%
	out << perf.all[size - margin] << "\t";

	//max
	out << perf.max;

	return out;
}
