/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// Implement a memory source with behave as a cache by keeping macro blocs into memory
/// to reduce exchanges with the OS and pay less the price of first touch page
/// faults.

//import
use common::types::*;
use common::list::*;
use common::shared::*;

/// Implement the header to track state of free macro blocs we keep in the cache.
struct FreeMacroBloc {
    node: ListNode,
    total_size: Size,
}

type FreeMacroBlocList = List<FreeMacroBloc>;

impl FreeMacroBloc {
    pub fn new(addr: Addr, total_size: Size) -> SharedPtrBox<FreeMacroBloc> {
        let mut ptr: SharedPtrBox<FreeMacroBloc> = SharedPtrBox::new_addr(addr);
        *ptr.get_mut() = FreeMacroBloc {
            node: ListNode::new(),
            total_size: total_size,
        };
        ptr
    }

    pub fn get_total_size(&self) -> Size {
        self.total_size
    }
}

impl Listable<FreeMacroBloc> for FreeMacroBloc {
    fn get_list_node<'a>(&'a self) -> &'a ListNode {
        &self.node
    }

    fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
        &mut self.node
    }

    fn get_from_list_node<'a>(elmt: * const ListNode) -> * const FreeMacroBloc {
        unsafe{&*(elmt as * const ListNode as Addr as * const FreeMacroBloc)}
    }

    fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut FreeMacroBloc {
        unsafe{&mut *(elmt as * mut ListNode as Addr as * mut FreeMacroBloc)}
    }
}
