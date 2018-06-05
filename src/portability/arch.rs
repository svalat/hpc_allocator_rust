/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module provide functions depening on architecture like inline ASM
use common::types::Size;

/// Implement a fast log by using asm direct operation for x86_64
#[cfg(any(target_arch = "x86" ,target_arch = "x86_64"))]
pub fn fast_log_2(size: Size) -> Size {
	let mut res = 0;
	let mut size = size;
	if size == 0 {
		return 0;
	} else {
		unsafe{
			asm!("bsr $1, $0":"=r" (res):"r"(size));
		};
		return res;
	}
}

/// Implementation of log 2 for generic arch, use fallback
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub fn fast_log_2(size: Size) -> Size {
	slow_generic_log_2(size)
}

/// Fallback implementation in pure rust, no asm.
pub fn slow_generic_log_2(size: Size) -> Size {
	let mut size = size;
	let mut res = 0;
	
	while size > 1 {
		size = size >> 1 ; 
		res += 1;
	}

	res
}

#[cfg(test)]
mod tests
{
	use portability::arch;
	use common::types::Size;

	#[test]
	fn fast_log_2() {
		for i in 16..2*1024*1024 {
			assert_eq!((i,arch::fast_log_2(i)), (i,arch::slow_generic_log_2(i)));
		}
	}
}
