/*****************************************************
*            PROJECT  : MPC_Allocator_CPP            *
*            VERSION  : 0.0.0                        *
*            DATE     : 07/2013                      *
*            AUTHOR   : Valat Sébastien              *
*            LICENSE  : CeCILL-C                     *
*****************************************************/

#ifndef POSIX_ALLOCATOR_STD_H
#define POSIX_ALLOCATOR_STD_H

/********************  HEADERS  *********************/
#include <cstdlib>

/********************  NAMESPACE  *******************/
namespace MPCAllocator
{

/*********************  CLASS  **********************/
class PosixAllocatorStd
{
	public:
		void postInit(void);
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
		//extra functions
		size_t getInnerSize ( void* ptr );
		size_t getRequestedSize ( void* ptr );
		size_t getTotalSize ( void* ptr );
		//compat with glibc
	private:
		template <class T> void loadFunction(T & func,const char * name);
	private:
		void   (*libc_free) ( void* ptr );
		void * (*libc_malloc) ( size_t size );
		void * (*libc_realloc) ( void* ptr, size_t size );
		void * (*libc_calloc)(size_t nmemb, size_t size);
		int    (*libc_posix_memalign)(void **memptr, size_t alignment, size_t size);
		void * (*libc_aligned_alloc)(size_t alignment, size_t size);
		void * (*libc_valloc)(size_t size);
		void * (*libc_memalign)(size_t alignment, size_t size);
		void * (*libc_pvalloc)(size_t size);
		int isDlsym;
		char dlsymBuffer[4096];
};

};

#endif //STD_POSIX_ALLOCATOR_H
