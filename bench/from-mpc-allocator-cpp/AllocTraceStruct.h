/*****************************************************
*            PROJECT  : MPC_Allocator_CPP            *
*            VERSION  : 0.0.0                        *
*            DATE     : 07/2013                      *
*            AUTHOR   : Valat Sébastien              *
*            LICENSE  : CeCILL-C                     *
*****************************************************/

#ifndef ALLOC_TRACE_STRUC_H
#define ALLOC_TRACE_STRUC_H

/********************  HEADERS  *********************/
#include <cstdlib>
#include <stdint.h>
#include "../from-fftw/cycle.h"

/********************  NAMESPACE  *******************/
namespace MPCAllocator
{

/********************  ENUM  ************************/
enum TraceEntryType
{
	TRACE_MALLOC,
	TRACE_FREE,
};

/*********************  STRUCT  *********************/
struct TraceEntry
{
	size_t size;
	union {
		void * ptr;
		size_t ptrIndex;
	} ptrInfo;
	uint16_t threadId;
	uint16_t type;
	uint32_t padding;
};

};

#endif //ALLOC_TRACE_STRUC_H
