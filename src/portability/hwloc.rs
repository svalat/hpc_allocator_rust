/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module impelment the basic wrapper to memory management functions

//import
extern crate libc;

//declare const
//hwloc_obj_type_t
pub const HWLOC_OBJ_SYSTEM: libc::c_int = 0;
pub const HWLOC_OBJ_MACHINE: libc::c_int = 1;
pub const HWLOC_OBJ_NUMANODE: libc::c_int = 2;
//hwloc_get_type_depth_e
pub const HWLOC_TYPE_DEPTH_UNKNOWN: libc::c_int = -1;

//declare type
type hwloc_topology_t = * mut libc::c_void;
type hwloc_obj_type_t = libc::c_int;
type hwloc_obj_t = * mut libc::c_void;
type hwloc_bitmap_t = * mut libc::c_void;
type hwloc_const_bitmap_t = * mut libc::c_void;
type hwloc_nodeset_t = hwloc_bitmap_t;
type hwloc_cpuset_t = hwloc_bitmap_t;
type hwloc_const_cpuset_t = hwloc_cpuset_t;
type hwloc_membind_policy_t = libc::c_int;

//declare extern funcs
extern {
	fn hwloc_topology_init(topology:*mut hwloc_topology_t) -> libc::c_int;
	fn hwloc_topology_load(topology: hwloc_topology_t) -> libc::c_int;
	fn hwloc_topology_destroy(topology: hwloc_topology_t);
	fn hwloc_get_nbobjs_by_type (topology: hwloc_topology_t, obj_type: hwloc_obj_type_t) -> libc::c_int;
	fn hwloc_bitmap_alloc() -> hwloc_bitmap_t;
	fn hwloc_get_membind_nodeset(topology: hwloc_topology_t, nodeset: hwloc_nodeset_t, policy: * mut hwloc_membind_policy_t, flags: libc::c_int) -> libc::c_int;
	fn hwloc_get_membind(topology: hwloc_topology_t, set: hwloc_bitmap_t, policy: * mut hwloc_membind_policy_t, flags: libc::c_int) -> libc::c_int;
	fn hwloc_get_type_depth(topology: hwloc_topology_t, tpye: hwloc_obj_type_t) -> libc::c_int;
	fn hwloc_bitmap_iszero(bitmap: hwloc_const_bitmap_t) -> libc::c_int;
	fn hwloc_bitmap_zero(bitmap: hwloc_bitmap_t);
	fn hwloc_bitmap_fill(bitmap: hwloc_bitmap_t);
	fn hwloc_bitmap_weight(bitmap: hwloc_const_bitmap_t) -> libc::c_int;
	fn hwloc_bitmap_free(bitmap: hwloc_bitmap_t);
	fn hwloc_get_cpubind(topology: hwloc_topology_t, set: hwloc_cpuset_t, flags: libc::c_int) -> libc::c_int;
	fn hwloc_bitmap_last(bitmap: hwloc_const_bitmap_t) -> libc::c_int;
	fn hwloc_bitmap_first(bitmap: hwloc_const_bitmap_t) -> libc::c_int;
	fn hwloc_bitmap_isset(bitmap: hwloc_const_bitmap_t, id: libc::c_uint) -> libc::c_int;
	fn hwloc_bitmap_next(bitmap: hwloc_const_bitmap_t, id: libc::c_int) -> libc::c_int;
	fn hwloc_topology_get_depth(topology: hwloc_topology_t) -> libc::c_uint;
	fn hwloc_get_depth_type (topology: hwloc_topology_t, depth: libc::c_uint) -> hwloc_obj_type_t;
}

//this is inlined function in hwloc header
fn hwloc_cpuset_to_nodeset(topology: hwloc_topology_t, cpuset: hwloc_const_cpuset_t, nodeset: hwloc_nodeset_t) {
	unsafe{
		let depth = hwloc_get_type_depth(topology, HWLOC_OBJ_NUMANODE);
		if depth == HWLOC_TYPE_DEPTH_UNKNOWN {
			if hwloc_bitmap_iszero(cpuset) != 0 {
				hwloc_bitmap_zero(nodeset);
			} else {
				hwloc_bitmap_fill(nodeset);
			}
		}
	}
}

fn hwloc_get_nbobjs_inside_cpuset_by_depth (topology: hwloc_topology_t, set: hwloc_const_cpuset_t, depth: libc::c_uint) -> libc::c_uint
{
	/*hwloc_obj_t obj = hwloc_get_obj_by_depth (topology, depth, 0);
	unsigned count = 0;
	if (!obj || !obj->cpuset)
		return 0;
	while (obj) {
		if (!hwloc_bitmap_iszero(obj->cpuset) && hwloc_bitmap_isincluded(obj->cpuset, set))
		count++;
		obj = obj->next_cousin;
	}
	return count;*/
}

#[cfg(test)]
mod tests
{
	use common::consts::*;
	use portability::hwloc;

	/*#[test]
	fn test_mmap_mremap_munap() {
	
	}*/
}