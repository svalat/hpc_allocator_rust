# HPC Allocator Rust

[![Build Status](https://travis-ci.org/svalat/hpc_allocator_rust.svg?branch=master)](https://travis-ci.org/svalat/hpc_allocator_rust)

It is a small project to try to push rust in his limit and better understant the low level aspects of the language.

This is a reimplemenation of the C version of MPC\_Allocator embeded into the MPC framework (http://mpc.hpcframework.paratools.com/).
This memory allocator provides :

 * Support of NUMA architecture. For binded threads it automatically isolate the data transfers between each NUMA nodes. Your threads need to be binded
 before first call to malloc or call `mpc_alloc_numa_rebind` after binding your threads.
 * Large buffer caching and reuse. For HPC application it is an issue to return too much the memory to the OS due to performance issue,
 MPC allocator reuse them as much as possible with some caching technics and mremap usage to avoid large fragmentation.
 * Isolation of thread sub-allocators. Each thread run his own lock free allocator.
 * Fully lock free implementation of free method. It uses a special mostly lock-free register to find the allocator linked to each segment.
 * Manage remote free (returning a segment to another thread) with a dedicated lock-free list.
 * Management of small chunk uses the same approach than Jemalloc (http://www.canonware.com/jemalloc/) with bitfield headers and size segregation.
 * Medium chunks are handled by segregeted double linked list for fast merging.
 
Due to it's deseign this allocator might by default consume more memory to prevend to much exchange with the OS. This can be controled by.... [[FIXME]]...

**WARNING**: This version is not yet stable, you can get the C stable version from MPC (http://mpc.hpcframework.paratools.com/) 
looking into `mpcframework/MPC_Allocator`. It can be built outside of MPC.

This version contain most of the features from the C version (and more), it is not yet tunned but it 
is easier to read than the C version if you want to understand how it works.

## Detailed research documentation

If you want more details about the research work behind this allocator you can read my PhD. :

 * PhD. manuscript (in french) : https://tel.archives-ouvertes.fr/tel-01253537
 * PhD. defense slides (in english) : http://fr.slideshare.net/SbastienValat1/2014-valatphddefenseslides
 * A reserch paper about page zeroing performance issue : http://doi.acm.org/10.1145/2492408.2492414

## Other parallel allocators

If you search other good parallel memory allocator I studied for this work :

 * Jemalloc (http://www.canonware.com/jemalloc/)
 * TCMalloc (https://github.com/gperftools/gperftools)
 * Lokless allocator (http://locklessinc.com/downloads/)
 * Hoard (http://www.hoard.org/)

## Authors and licence

This allocator is distributed under CeCILL-C has it's C counterpart.

This was developped by Sébastien Valat. A special thanks to Marc Pérache and William Jalby to
advice the PhD. leading to the original C version of this allocator.