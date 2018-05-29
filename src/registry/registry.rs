/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This implement the region registry to keep track of all the allocated segments
///and map their related chunk manager.

//import
use common::consts::*;
use registry::region::*;

///Define the global registry
pub struct RegionRegistry {
	regions: [* mut Region; MAX_REGIONS],
}
