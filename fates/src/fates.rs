#![allow(dead_code)]
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type FateFn<T> = dyn Fn() -> T + Send + Sync + 'static;

pub trait FateTrait: Send + Sync {
    fn is_dirty(&self) -> bool;
    fn set_dirty(&self);
    fn add_dependent(&self, dependent: Box<dyn FateTrait>);
    fn remove_dependent(&self, dependent: Box<dyn FateTrait>);
    fn get_id(&self) -> usize;
}

enum Binding<T> {
    Value(T),
    Expression(Box<FateFn<T>>),
}

impl<T: Default> Default for Binding<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

#[derive(Default)]
struct FateInternal<T: Clone> {
    value: Binding<T>,
    dependencies: Vec<Box<dyn FateTrait>>,
    dependents: Vec<Box<dyn FateTrait>>,
}

#[derive(Default, Clone)]
pub struct Fate<T: Clone> {
    cached_value: Arc<RwLock<T>>,
    dirty: Arc<AtomicBool>,
    data: Arc<RwLock<FateInternal<T>>>,
}

impl<T: 'static + Clone + Send + Sync> FateTrait for Fate<T> {
    fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    fn set_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    fn add_dependent(&self, dependent: Box<dyn FateTrait>) {
        let mut data = self.data.write();
        data.dependents.push(dependent);
    }

    fn remove_dependent(&self, dependent: Box<dyn FateTrait>) {
        let mut data = self.data.write();
        let index = data
            .dependents
            .iter()
            .position(|dep| dep.get_id() == dependent.get_id());
        if let Some(index) = index {
            data.dependents.remove(index);
        }
    }

    fn get_id(&self) -> usize {
        Arc::as_ptr(&self.data) as usize
    }
}

impl<T: 'static + Clone + Send + Sync> Fate<T> {
    pub fn get(&self) -> T {
        if self.is_dirty() {
            let data = self.data.write();
            let result = match &data.value {
                Binding::Value(value) => value.clone(),
                Binding::Expression(expression) => expression(),
            };
            let mut cached_value = self.cached_value.write();
            *cached_value = result.clone();
            result
        } else {
            self.cached_value.read().clone()
        }
    }

    pub fn bind_value(&self, value: T) {
        let mut data = self.data.write();
        data.value = Binding::Value(value);
        self.set_dirty();

        for dependent in &data.dependents {
            dependent.set_dirty();
        }
    }

    pub fn bind_expression(
        &self,
        expression: Box<FateFn<T>>,
        dependencies: Vec<Box<dyn FateTrait>>,
    ) {
        self.set_dependencies(dependencies);
        let mut data = self.data.write();
        data.value = Binding::Expression(expression);
        self.set_dirty();

        for dependent in &data.dependents {
            dependent.set_dirty();
        }
    }

    pub fn from_value(value: T) -> Fate<T> {
        Fate {
            cached_value: Arc::new(RwLock::new(value.clone())),
            dirty: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(FateInternal {
                value: Binding::Value(value),
                dependencies: vec![],
                dependents: vec![],
            })),
        }
    }

    pub fn from_expression(
        expression: Box<FateFn<T>>,
        dependencies: Vec<Box<dyn FateTrait>>,
    ) -> Fate<T> {
        let result = Fate {
            cached_value: Arc::new(RwLock::new(expression())),
            dirty: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(FateInternal {
                value: Binding::Expression(expression),
                dependencies: vec![],
                dependents: vec![],
            })),
        };
        result.set_dependencies(dependencies);
        result
    }

    fn clear_dependencies(&self) {
        self.remove_all_dependencies();
        let mut data = self.data.write();
        data.dependencies.clear();
    }

    fn remove_all_dependencies(&self) {
        let data = self.data.read();
        for dependency in &data.dependencies {
            dependency.remove_dependent(Box::new(self.clone()));
        }
    }

    fn set_dependencies(&self, dependencies: Vec<Box<dyn FateTrait>>) {
        self.remove_all_dependencies();
        let mut data = self.data.write();
        data.dependencies = dependencies;
        for dependency in &data.dependencies {
            dependency.add_dependent(Box::new(self.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Fate;
    use fates_macro::fate;
    use std::thread;

    #[test]
    fn simple() {
        let a = Fate::from_value(3);
        let b = Fate::from_value(5);
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c = Fate::from_expression(
            Box::new(move || a_clone.get() + b_clone.get()),
            vec![Box::new(a.clone()), Box::new(b.clone())],
        );
        assert_eq!(c.get(), 8);
        b.bind_value(100);
        assert_eq!(c.get(), 103);
    }

    #[test]
    fn less_simple() {
        let a = Fate::from_value(10);
        let b = Fate::from_value(23);
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c = Fate::from_expression(
            Box::new(move || a_clone.get() + b_clone.get() * b_clone.get()),
            vec![Box::new(a.clone()), Box::new(b.clone())],
        );
        assert_eq!(c.get(), 10 + 23 * 23);
        b.bind_value(113);
        assert_eq!(c.get(), 10 + 113 * 113);

        let c_clone = c.clone();
        let a_clone = a.clone();
        let d = Fate::from_expression(
            Box::new(move || c_clone.get() * a_clone.get()),
            vec![Box::new(c.clone()), Box::new(a.clone())],
        );

        assert_eq!(d.get(), (10 + 113 * 113) * 10);

        let a_clone = a.clone();
        let b_clone = b.clone();
        let e = Fate::from_value(2);
        let e_clone = e.clone();
        c.bind_expression(
            Box::new(move || a_clone.get() * b_clone.get() / e_clone.get()),
            vec![
                Box::new(a.clone()),
                Box::new(b.clone()),
                Box::new(e.clone()),
            ],
        );
        assert_eq!(c.get(), 10 * 113 / 2);
    }

    fn circular_reference() {
        let a = Fate::from_value(3);
        let b = Fate::from_value(5);
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c = Fate::from_expression(
            Box::new(move || a_clone.get() + b_clone.get()),
            vec![Box::new(a.clone()), Box::new(b.clone())],
        );
        let a_clone = a.clone();
        let c_clone = c.clone();
        b.bind_expression(
            Box::new(move || a_clone.get() + c_clone.get()),
            vec![Box::new(a.clone()), Box::new(c.clone())],
        );
    }

    #[test]
    fn macro_simple_test() {
        let a = 5;
        let b = a * 5;
        let c = a * b;
        fate! {
            [a2, b2, c2]
            let a2 = 5;
            let b2 = a2 * 5;
            let c2 = a2 * b2;
        }
        assert_eq!(a, a2.get());
        assert_eq!(b, b2.get());
        assert_eq!(c, c2.get());

        let a = 7;
        let b = a * 5;
        let c = a * b;

        fate! {a2 = 7;}
        assert_eq!(a, a2.get());
        assert_eq!(b, b2.get());
        assert_eq!(c, c2.get());
    }

    #[test]
    fn thread_safe_test() {
        fate! {
            [a, b]
            let a = 0;
            let b = a * 10;
        }

        let mut thread_handles = Vec::new();
        for _i in 1..100 {
            let a2: Fate<_> = a.clone();
            let b2: Fate<_> = b.clone();
            let handle = thread::spawn(move || {
                for i in 1..100 {
                    let value = 30;
                    fate! {[a2, b2] a2 = i + value;}
                    let _r = b2.get();
                    // note: this will not be the correct value because it is still
                    // being assigned to randomly, but no exceptions!
                }
            });
            thread_handles.push(handle);
        }

        for handle in thread_handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn value_capture_test() {
        let a = 5;
        let b = 10;
        fate! {
            [c, d, e]
            let c = 15; // Comment
            let d = a + b;
            let e = c * d;
        }

        assert_eq!(c.get(), d.get());
        assert_eq!(e.get(), c.get() * d.get());

        fate! {[c] c = a + b;}
        assert_eq!(c.get(), a + b);
        assert_eq!(e.get(), c.get() * d.get());
    }

    #[test]
    fn explicit_fate_test() {
        struct TestStruct {
            fate: Fate<i32>,
        }
        fate! {
            let a = 10;
        }
        let test_struct = TestStruct { fate: a.clone() };
        assert_eq!(a.get(), 10);
        fate! {a = 15;}
        assert_eq!(test_struct.fate.get(), 15);
    }

    #[test]
    fn mix_types_test() {
        fate! {
            [a,b,c]
            let a = "String".to_string();
            let b = 10;
            let c = a + " " + &b.to_string();
        }

        assert_eq!(&c.get(), "String 10");

        fate! {
            [a,b]
            a = "String2".to_string();
            b = 100;
        }

        assert_eq!(&c.get(), "String2 100");
    }
}
