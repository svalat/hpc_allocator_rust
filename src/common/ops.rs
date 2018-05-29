/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module implement basic operations which might be used
///everywhere in the code.

use common::types::{Size};

#[inline]
pub fn ceil_to_power_of_2(size:Size,align:Size) -> Size {
	size & !(align-1)
}

#[inline]
pub fn up_to_power_of_2(size:Size,align:Size) -> Size {
	let ret;

	if size & (align-1) != 0 {
		ret = (size & !(align-1)) + align;
	} else {
		ret = size;
	}

	ret
}

#[cfg(test)]
mod tests
{
	use common::ops;

	#[test]
	fn ceil_to_power_of_2() {
		assert_eq!(ops::ceil_to_power_of_2(0,1),0);
		assert_eq!(ops::ceil_to_power_of_2(9,1),9);
		
		assert_eq!(ops::ceil_to_power_of_2(0,2),0);
		assert_eq!(ops::ceil_to_power_of_2(9,2),8);
		assert_eq!(ops::ceil_to_power_of_2(10,2),10);

		assert_eq!(ops::ceil_to_power_of_2(0,8),0);
		assert_eq!(ops::ceil_to_power_of_2(9,8),8);
		assert_eq!(ops::ceil_to_power_of_2(10,8),8);
	}

	#[test]
	fn up_to_power_of_2() {
		assert_eq!(ops::up_to_power_of_2(0,1),0);
		assert_eq!(ops::up_to_power_of_2(9,1),9);
		
		assert_eq!(ops::up_to_power_of_2(0,2),0);
		assert_eq!(ops::up_to_power_of_2(9,2),10);
		assert_eq!(ops::up_to_power_of_2(10,2),10);

		assert_eq!(ops::up_to_power_of_2(0,8),0);
		assert_eq!(ops::up_to_power_of_2(9,8),16);
		assert_eq!(ops::up_to_power_of_2(10,8),16);
	}
}