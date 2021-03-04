/*****************************************************
*            PROJECT  : MPC_Allocator_CPP            *
*            VERSION  : 0.0.0                        *
*            DATE     : 07/2013                      *
*            AUTHOR   : Valat Sébastien              *
*            LICENSE  : CeCILL-C                     *
*****************************************************/

/********************  HEADERS  *********************/
#include <dlfcn.h>
#include <cassert>
#include <unistd.h>
#include "PosixAllocatorStd.h"

/********************  NAMESPACE  *******************/
namespace MPCAllocator
{

/*******************  FUNCTION  *********************/
void PosixAllocatorStd::postInit ( void )
{
	isDlsym = true;
	loadFunction(libc_calloc,"calloc");
	isDlsym = false;
	loadFunction(libc_aligned_alloc,"aligned_alloc");
	loadFunction(libc_free,"free");
	loadFunction(libc_malloc,"malloc");
	loadFunction(libc_memalign,"memalign");
	loadFunction(libc_posix_memalign,"posix_memalign");
	loadFunction(libc_pvalloc,"pvalloc");
	loadFunction(libc_realloc,"realloc");
	loadFunction(libc_valloc,"valloc");
}

/*******************  FUNCTION  *********************/
template <class T>
void PosixAllocatorStd::loadFunction ( T & func,const char* name )
{
	func = (T)dlsym(RTLD_NEXT,name);
	assert(func != NULL);
	//allocAssert(func != NULL);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::aligned_alloc ( size_t alignment, size_t size )
{
	return libc_aligned_alloc(alignment,size);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::calloc ( size_t nmemb, size_t size )
{
	if (isDlsym)
	{
		assert(sizeof(dlsymBuffer) >= nmemb*size);
		return dlsymBuffer;
	}

	return libc_calloc(nmemb,size);
}

/*******************  FUNCTION  *********************/
void PosixAllocatorStd::free ( void* ptr )
{
	libc_free(ptr);
}

/*******************  FUNCTION  *********************/
size_t PosixAllocatorStd::getInnerSize ( void* ptr )
{
	//allocFatal("Not supported");
	assert(false);
	return -1;
}

/*******************  FUNCTION  *********************/
size_t PosixAllocatorStd::getRequestedSize ( void* ptr )
{
	//allocFatal("Not supported");
	assert(false);
	return -1;
}

/*******************  FUNCTION  *********************/
size_t PosixAllocatorStd::getTotalSize ( void* ptr )
{
	//allocFatal("Not supported");
	assert(false);
	return -1;
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::malloc ( size_t size )
{
	return libc_malloc(size);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::memalign ( size_t alignment, size_t size )
{
	return libc_memalign(alignment,size);
}

/*******************  FUNCTION  *********************/
int PosixAllocatorStd::posix_memalign ( void** memptr, size_t alignment, size_t size )
{
	return libc_posix_memalign(memptr,alignment,size);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::pvalloc ( size_t size )
{
	return libc_pvalloc(size);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::realloc ( void* ptr, size_t size )
{
	return libc_realloc(ptr,size);
}

/*******************  FUNCTION  *********************/
void* PosixAllocatorStd::valloc ( size_t size )
{
	return libc_valloc(size);
}

};
