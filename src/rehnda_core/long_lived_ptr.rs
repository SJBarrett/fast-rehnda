use std::ops::Deref;

pub struct LongLivedObject<T> {
    object: Box<T>
}

impl<T> LongLivedObject<T> {
    pub fn new(t: T) -> LongLivedObject<T>{
        LongLivedObject {
            object: Box::new(t),
        }
    }

    pub fn ptr(&self) -> ConstPtr<T> {
        ConstPtr {
            ptr: &*self.object
        }
    }
}

impl<T> Deref for LongLivedObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.deref()
    }
}

pub struct ConstPtr<T> {
    ptr: *const T,
}

impl<T> Deref for ConstPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> Clone for ConstPtr<T> {
    fn clone(&self) -> Self {
        ConstPtr {
            ptr: self.ptr,
        }
    }
}

impl<T> Copy for ConstPtr<T> {
}

unsafe impl<T> Send for ConstPtr<T> {
}

unsafe impl<T> Sync for ConstPtr<T> {
}