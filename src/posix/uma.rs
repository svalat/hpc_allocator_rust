/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file implement the UMA allocator considering a local allocator for 
///every thread and using TLS (Thread Local Storage) to keep track of them.

//import
use common::traits::Allocator;
