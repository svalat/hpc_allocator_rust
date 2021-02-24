/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///Import pthread_spinlock from C libray on posix systems
///Need to look if at some point it will appear in crate libc...

//import
extern crate libc;

//import
use core::ops::{Drop, Deref, DerefMut};

/// Define the libnuma bitmask
pub struct bitmask {
	size: libc::c_ulong,
	maskp: * mut libc::c_ulong,
}

// requiered functions
extern {
	fn numa_num_task_nodes() -> libc::c_int;
	fn numa_preferred() -> libc::c_int;
}

/// Determine the best NUMA node for the current task. Return -1 if there is
/// multiple NUMA node the task can run on.
pub fn numa_detect_affinity() -> i32 {
	// if more than one node
	if unsafe{numa_num_task_nodes()} > 1 {
		return -1;
	}

	//preferred
	let preferred = unsafe{numa_preferred()};
	if preferred >= 0 {
		return preferred;
	}

	//fail
	panic!("Preferred node is -1, cannot determine which NUMA node to use !");
}

#[cfg(test)]
mod tests
{
	extern crate std;
	use portability::libnuma::*;

	#[test]
	fn test_numa_preferred() {
		assert_ne!(-1, unsafe{numa_preferred()});
	}

	#[test]
	fn test_numa_num_task_nodes() {
		assert_ne!(0, unsafe{numa_num_task_nodes()});
	}

	#[test]
	fn test_numa_detect_affinity() {
		let res = numa_detect_affinity();
		assert!(res > -1);
	}
}
