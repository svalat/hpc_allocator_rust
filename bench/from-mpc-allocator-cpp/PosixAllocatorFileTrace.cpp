/*****************************************************
*            PROJECT  : MPC_Allocator_CPP            *
*            VERSION  : 0.0.0                        *
*            DATE     : 07/2013                      *
*            AUTHOR   : Valat Sébastien              *
*            LICENSE  : CeCILL-C                     *
*****************************************************/

/********************  HEADERS  *********************/
#include <cstdio>
#include <cassert>
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <libgen.h>
#include "PosixAllocatorFileTrace.h"

#define allocUnused(x)

/********************  NAMESPACE  *******************/
namespace MPCAllocator
{

/*********************  CONSTS  *********************/
static const char cstTraceFilename[] = "alloc-trace-%s-%08d.raw";

/********************  GLOBALS  *********************/
static __thread int gblThreadId = 0;
static bool gblInit = false;
static __thread bool isFromLocalCall = true;

/*******************  FUNCTION  *********************/
PosixAllocatorFileTrace::PosixAllocatorFileTrace ( PosixAllocatorStd * allocator ) 
{
	//basic setup
	this->allocator = allocator;
	this->index = 0;

	//allocate tmp buffer
	char fname[2*sizeof(cstTraceFilename)];
	
	//errors
	assert(gblInit == false);
	nextThreadId = 1;

	//get pid
	int pid = getpid();

	//get exe name
	char exePath[4096];
	readlink("/proc/self/exe", exePath, sizeof(exePath));

	//build fname
	size_t res = sprintf(fname,cstTraceFilename,basename(exePath),pid);
	assert(res <= sizeof(fname));

	//open file
	printf("Trace allocator in raw file %s\n",fname);
	fd = open(fname,O_TRUNC|O_WRONLY|O_CREAT,S_IRUSR|S_IRGRP|S_IROTH|S_IWUSR);
	assert(fd >= 0);
	
	//mark init ok (for debug)
	gblInit = true;

	//unused vars for assert
	allocUnused(res);

	//mark instr started
	isFromLocalCall = false;
}

/*******************  FUNCTION  *********************/
void PosixAllocatorFileTrace::writeEvent ( TraceEntry& entry, TraceEntryType type )
{
	//init
	entry.type = type;
	entry.threadId = gblThreadId;
	
	//check for threadId init
	if (entry.threadId <= 0)
	{
		printf("Alloc trace new thread %d\n",nextThreadId);
		entry.threadId = nextThreadId++;
		gblThreadId = entry.threadId;
	}
	
	//write
	assert(fd >= 0);
	size_t status = write(fd,&entry,sizeof(entry));
	assert(status == sizeof(entry));

	//unused vars for assert
	allocUnused(status);
}

/*******************  FUNCTION  *********************/
void PosixAllocatorFileTrace::writeAnswer ( void* res )
{
	//tmp
	uint64_t formatedRes = (uint64_t)res;

	//write
	assert(fd >= 0);
	size_t status = write(fd,&formatedRes,sizeof(formatedRes));
	assert(status == sizeof(formatedRes));

	//unsued variables for assert
	allocUnused(status);
}

/*******************  FUNCTION  *********************/
void PosixAllocatorFileTrace::onMalloc(void * ptr, size_t size)
{
	//generate
	TraceEntry event;
	
	//check
	assert(gblInit);

	//skip
	if (isFromLocalCall)
		return;
	isFromLocalCall = true;
	
	//special info
	event.size = size;
	event.ptrInfo.ptr = NULL;

	lock.lock();		
		ptrToIndexMap[ptr] = index;
		writeEvent(event,TRACE_MALLOC);
		index++;
	lock.unlock();

	isFromLocalCall = false;
}

/*******************  FUNCTION  *********************/
void PosixAllocatorFileTrace::onFree(void * ptr)
{
	//generate
	TraceEntry event;
	
	//check
	assert(gblInit);

	if (ptr == NULL)
		return;
	if (isFromLocalCall)
		return;
	isFromLocalCall = true;
	
	//special info
	event.size = 0;

	lock.lock();		
		auto it = ptrToIndexMap.find(ptr);
		if (it != ptrToIndexMap.end()) {
			event.ptrInfo.ptrIndex = it->second;
			ptrToIndexMap.erase(it);
			writeEvent(event,TRACE_FREE);
			index++;
		}
	lock.unlock();

	isFromLocalCall = false;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::malloc ( size_t size )
{
	void * ptr = allocator->malloc(size);
	onMalloc(ptr, size);
	return ptr;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::aligned_alloc ( size_t alignment, size_t size )
{
	void * ptr = allocator->aligned_alloc(alignment,size);
	onMalloc(ptr, size);
	return ptr;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::calloc ( size_t nmemb, size_t size )
{
	void * ptr = allocator->calloc(nmemb,size);
	onMalloc(ptr, nmemb * size);
	return ptr;
}

/*******************  FUNCTION  *********************/
void PosixAllocatorFileTrace::free ( void* ptr )
{
	onFree(ptr);
	allocator->free(ptr);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::memalign ( size_t alignment, size_t size )
{
	void * ptr = allocator->memalign(alignment,size);
	onMalloc(ptr, size);
	return ptr;
}

/*******************  FUNCTION  *********************/
int PosixAllocatorFileTrace::posix_memalign ( void** memptr, size_t alignment, size_t size )
{
	int res = allocator->posix_memalign(memptr,alignment,size);
	onMalloc(*memptr, size);
	return res;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::realloc ( void* old_ptr, size_t size )
{
	void * new_ptr = allocator->realloc(old_ptr, size);
	onFree(old_ptr);
	onMalloc(new_ptr, size);
	return new_ptr;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::pvalloc ( size_t size )
{
	void * ptr = allocator->pvalloc(size);
	onMalloc(ptr, size);
	return ptr;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorFileTrace::valloc ( size_t size )
{
	void * ptr = allocator->valloc(size);
	onMalloc(ptr, size);
	return ptr;
}

};
