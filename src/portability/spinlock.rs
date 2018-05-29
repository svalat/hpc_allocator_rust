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

//low level spincloks (pthread_spinlock_t) are int (from /usr/include/bits/pthreadtypes.h)
type pthread_spin_lock_t = libc::c_int;
const PTHREAD_PROCESS_PRIVATE: libc::c_int = 0;

//declare extern funcs
extern {
	fn pthread_spin_init(lock: * const pthread_spin_lock_t, pshared:libc::c_int) -> libc::c_int;
	fn pthread_spin_lock(lock: * const pthread_spin_lock_t) -> libc::c_int;
	fn pthread_spin_unlock(lock: * const pthread_spin_lock_t) -> libc::c_int;
	fn pthread_spin_destroy(lock: * const pthread_spin_lock_t) -> libc::c_int;
}

//built object to hide the lowlevel funcs
pub struct SpinLock<T> {
	lock: pthread_spin_lock_t,
	data: T,
}

pub struct SpinLockGuard<'a, T:'a>
{
    lock: &'a pthread_spin_lock_t,
    data: &'a mut T,
}

impl <T> SpinLock<T> {
	///Construct the spinlock and embed the content in it
	pub fn new(obj: T) -> Self {
		let ret = Self {
			lock: 0,
			data: obj, 
		};

		let ptr = &ret.lock as * const pthread_spin_lock_t;
		let status = unsafe{pthread_spin_init(ptr,PTHREAD_PROCESS_PRIVATE)};
		if status != 0 {
			panic!("Fail to init pthread spinlock !");
		}
		
		ret
	}

	///lock
 	pub fn lock(&self) -> SpinLockGuard<T>
    {
		let ptr = &self.lock as * const pthread_spin_lock_t;
        unsafe{pthread_spin_lock(ptr)};
        SpinLockGuard
        {
            lock: &self.lock,
            data: unsafe{&mut *(&self.data as * const T as * mut T)},
        }
    }

	///no lock
	pub fn nolock_safe_read<'a>(&'a self) -> &'a T {
		unsafe{&mut *(&self.data as * const T as * mut T)}
	}
}

impl<'a, T> Deref for SpinLockGuard<'a, T>
{
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T { &*self.data }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T>
{
    fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
}

impl<'a, T> Drop for SpinLockGuard<'a, T>
{
    fn drop(&mut self)
    {
        let ptr = self.lock as * const pthread_spin_lock_t;
        unsafe{pthread_spin_unlock(ptr)};
    }
}

#[cfg(test)]
mod tests
{
	extern crate std;

	use portability::spinlock::*;

	#[test]
	fn serial() {
		let spin = SpinLock::new(0);
		*spin.lock() += 1;
		*spin.lock() += 1;
		*spin.lock() += 1;
		assert_eq!(*spin.lock(), 3);
	}

	#[test]
	fn threads() {
		/*let spin = SpinLock::new(0);
		let mut handlers: std::Vec<JoinHandle<i32>>;
		
		for i in 0..128 {
			handlers[i] = std::thread::spawn( || {
				*spin.lock() += 1;
				0
			});
		}

		for i in 0..128 {
			handlers[i].join();
		}*/
	}
}