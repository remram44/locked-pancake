use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GcFlag(u8);

pub trait Traceable: 'static {
    fn trace(&self, flag: GcFlag);
}

pub trait GcAllocator {
    fn alloc<T: Traceable>(&mut self, value: T) -> GcRef<T>;

    fn mark<T: Traceable>(&self, gc_ref: GcRef<T>);
    fn sweep(&mut self);
}

pub struct SimpleGcAllocator {
    flag: GcFlag,
    pub(crate) values: Vec<NonNull<dyn ValueTrait>>,
}

impl Default for SimpleGcAllocator {
    fn default() -> SimpleGcAllocator {
        SimpleGcAllocator {
            flag: GcFlag(1),
            values: Vec::new(),
        }
    }
}

trait ValueTrait {
    fn dealloc(&mut self, flag: GcFlag) -> bool;
}

impl GcAllocator for SimpleGcAllocator {
    fn alloc<T: Traceable>(&mut self, value: T) -> GcRef<T> {
        // Allocate value and GcValue wrapper
        let gc_value = GcValue {
            value: Some(value),
            flag: RefCell::new(GcFlag(0)),
        };
        let ptr: &mut GcValue<T> = Box::leak(Box::new(gc_value));

        // Store reference in vec
        let ptr_t: NonNull<dyn ValueTrait> = {
            let r: &dyn ValueTrait = ptr;
            r.into()
        };
        self.values.push(ptr_t.into());

        // Return GcRef
        GcRef {
            ptr: ptr.into(),
            phantom: PhantomData,
        }
    }

    fn mark<T: Traceable>(&self, gc_ref: GcRef<T>) {
        gc_ref.trace_ref(self.flag);
    }

    fn sweep(&mut self) {
        // Sweep
        let flag = self.flag;
        self.values.retain(|v| {
            let v: &mut dyn ValueTrait = unsafe { &mut *v.as_ptr() };
            let deleted = v.dealloc(flag);
            !deleted
        });

        // Switch flag
        self.flag = GcFlag(match self.flag.0 {
            1 => 2,
            _ => 1,
        });
    }
}

pub struct GcValue<T: Traceable> {
    value: Option<T>,
    flag: RefCell<GcFlag>,
}

impl<T: Traceable> ValueTrait for GcValue<T> {
    fn dealloc(&mut self, flag: GcFlag) -> bool {
        if *self.flag.borrow() != flag {
            self.value.take();
            true
        } else {
            false
        }
    }
}

pub struct GcRef<T: Traceable> {
    ptr: NonNull<GcValue<T>>,
    phantom: PhantomData<T>,
}

impl<T: Traceable> Clone for GcRef<T> {
    fn clone(&self) -> GcRef<T> {
        GcRef {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: Traceable> GcRef<T> {
    pub fn trace_ref(&self, flag: GcFlag) {
        let gc_value = self.inner();
        let mut flag_ref = gc_value.flag.borrow_mut();
        if *flag_ref != flag {
            *flag_ref = flag;
            if let Some(ref value) = gc_value.value {
                value.trace(flag);
            }
        }
    }

    fn inner(&self) -> &GcValue<T> {
        unsafe {
            &*self.ptr.as_ptr()
        }
    }
}

impl<T: Traceable> Deref for GcRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let gc_value = self.inner();
        match gc_value.value {
            Some(ref v) => v,
            None => panic!("Attempt to dereference freed GcRef"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GcAllocator, GcFlag, GcRef, SimpleGcAllocator, Traceable};

    enum Value {
        Integer(i32),
        Array(Vec<GcRef<Value>>),
    }

    impl Traceable for Value {
        fn trace(&self, flag: GcFlag) {
            match self {
                Value::Integer(_) => {}
                Value::Array(v) => {
                    for elem in v {
                        elem.trace_ref(flag);
                    }
                }
            }
        }
    }

    #[test]
    fn test_gc() {
        let mut gc: SimpleGcAllocator = Default::default();
        let int1 = gc.alloc(Value::Integer(1));
        let int2 = gc.alloc(Value::Integer(2));
        let int3 = gc.alloc(Value::Integer(3));
        let _int4 = gc.alloc(Value::Integer(4));
        let arr1 = gc.alloc(Value::Array(vec![int1.clone()]));
        let _arr2 = gc.alloc(Value::Array(vec![int1.clone(), int2.clone()]));

        // Mark & sweep
        gc.mark(arr1.clone());
        gc.mark(int3.clone());
        gc.sweep();

        assert_eq!(gc.values.len(), 3);
        assert_eq!(
            gc.values
                .iter()
                .map(|v| v.as_ptr() as *const u8)
                .collect::<Vec<_>>(),
            vec![int1, int3, arr1]
                .iter()
                .map(|v| v.ptr.as_ptr() as *const u8)
                .collect::<Vec<_>>(),
        );
    }
}
