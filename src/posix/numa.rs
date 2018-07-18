/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file implement the NUMA allocator considering a local allocator for 
///every thread and using TLS (Thread Local Storage) to keep track of them.
///It also aigned the right memory source (NUMA based) to the local allocator
///looking on thread bindings.

//import
use common::traits::Allocator;
