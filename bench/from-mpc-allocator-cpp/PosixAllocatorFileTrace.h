/*****************************************************
*            PROJECT  : MPC_Allocator_CPP            *
*            VERSION  : 0.0.0                        *
*            DATE     : 07/2013                      *
*            AUTHOR   : Valat Sébastien              *
*            LICENSE  : CeCILL-C                     *
*****************************************************/

#ifndef POSIX_ALLOCATOR_FILE_TRACE_H
#define POSIX_ALLOCATOR_FILE_TRACE_H

/********************  HEADERS  *********************/
#include <mutex>
#include <map>
#include "AllocTraceStruct.h"
#include "PosixAllocatorStd.h"

/********************  NAMESPACE  *******************/
namespace MPCAllocator
{

/*********************  CLASS  **********************/
class PosixAllocatorFileTrace : public PosixAllocatorStd
{
	public:
		PosixAllocatorFileTrace(PosixAllocatorStd * allocator);
		//The posix interface
		void   free ( void* ptr );
		void * malloc ( size_t size );
		void * realloc ( void* ptr, size_t size );
		void * calloc(size_t nmemb, size_t size);
		int    posix_memalign(void **memptr, size_t alignment, size_t size);
		void * aligned_alloc(size_t alignment, size_t size);
		void * valloc(size_t size);
		void * memalign(size_t alignment, size_t size);
		void * pvalloc(size_t size);
	private:
		void onMalloc(void * ptr, size_t size);
		void onFree(void * ptr);
		void writeEvent(TraceEntry & entry, TraceEntryType type);
		void writeAnswer(void * res = NULL);
	private:
		int fd;
		std::mutex lock;
		int nextThreadId;
		size_t index;
		std::map<void*, size_t> ptrToIndexMap;
	private:
		PosixAllocatorStd * allocator;
};

};

#endif //POSIX_ALLOCATOR_FILE_TRACE_H
