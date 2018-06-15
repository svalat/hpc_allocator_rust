/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module implement the small chunk management using
/// a bitmap approach with runs and containers just like what is
/// done in JeMalloc.

/// export
pub mod run;
pub mod container;