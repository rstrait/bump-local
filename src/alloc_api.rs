use alloc::alloc::Layout;
use core::ptr::NonNull;

#[cfg(feature = "allocator_api")]
pub use core::alloc::{AllocError, Allocator};

#[cfg(all(feature = "allocator-api2", not(feature = "allocator_api")))]
pub use allocator_api2::alloc::{AllocError, Allocator};

use crate::Bump;

unsafe impl Allocator for Bump {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.local().as_inner().allocate(layout)
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe {
            self.local().as_inner().deallocate(ptr, layout);
        }
    }

    #[inline]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.local().as_inner().shrink(ptr, old_layout, new_layout) }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.local().as_inner().grow(ptr, old_layout, new_layout) }
    }

    #[inline]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.local().as_inner().grow_zeroed(ptr, old_layout, new_layout) }
    }
}
