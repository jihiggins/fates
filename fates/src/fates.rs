#![allow(dead_code)]
use std::cell::RefCell;
use std::rc::Rc;

type FateFn<T> = dyn Fn() -> T;

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
    dependencies: Vec<Fate<T>>,
    dependents: Vec<Fate<T>>,
}

#[derive(Default, Clone)]
pub struct Fate<T: Clone> {
    cached_result: Rc<RefCell<T>>,
    data: Rc<RefCell<FateInternal<T>>>,
}

impl<T: Clone> Fate<T> {
    fn from_value(value: T) -> Fate<T> {
        Fate {
            cached_result: Rc::new(RefCell::new(value.clone())),
            data: Rc::new(RefCell::new(FateInternal {
                value: Binding::Value(value),
                dependencies: vec![],
                dependents: vec![],
            })),
        }
    }

    fn from_expression(
        expression: Box<FateFn<T>>,
        dependencies: Vec<Fate<T>>,
    ) -> Fate<T> {
        let result = Fate {
            cached_result: Rc::new(RefCell::new(expression())),
            data: Rc::new(RefCell::new(FateInternal {
                value: Binding::Expression(expression),
                dependencies: vec![],
                dependents: vec![],
            })),
        };
        result.set_dependencies(dependencies);
        result
    }

    pub fn get(&self) -> T {
        self.cached_result.borrow().clone()
    }

    fn bind_value(&self, value: T) {
        {
            let mut data = self.data.borrow_mut();
            data.value = Binding::Value(value);
        }
        self.update();
    }

    fn bind_expression(&self, expression: Box<FateFn<T>>, dependencies: Vec<Fate<T>>) {
        {
            self.set_dependencies(dependencies);
            let mut data = self.data.borrow_mut();
            data.value = Binding::Expression(expression);
        }
        self.update();
    }

    fn update(&self) {
        let data = self.data.try_borrow_mut().expect("Circular reference");
        let result = match &data.value {
            Binding::Value(value) => value.clone(),
            Binding::Expression(expression) => expression(),
        };

        {
            let mut cached_result = self.cached_result.borrow_mut();
            *cached_result = result;
        }

        for dependent in &data.dependents {
            dependent.update();
        }
    }

    fn clear_dependencies(&self) {
        self.remove_all_dependencies();
        let mut data = self.data.borrow_mut();
        data.dependencies.clear();
    }

    fn remove_all_dependencies(&self) {
        let data = self.data.borrow();
        for dependency in &data.dependencies {
            dependency.remove_dependent(&self);
        }
    }

    fn set_dependencies(&self, dependencies: Vec<Fate<T>>) {
        self.remove_all_dependencies();
        let mut data = self.data.borrow_mut();
        data.dependencies = dependencies;
        for dependency in &data.dependencies {
            dependency.add_dependent(self);
        }
    }

    fn add_dependent(&self, dependent: &Fate<T>) {
        let mut data = self.data.borrow_mut();
        data.dependents.push(dependent.clone());
    }

    fn remove_dependent(&self, dependent: &Fate<T>) {
        let mut data = self.data.borrow_mut();
        let index = data
            .dependents
            .iter()
            .position(|dep| dep.data.as_ptr() == dependent.data.as_ptr());
        if let Some(index) = index {
            data.dependents.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Fate;
    use fates_macro::fate;

    #[test]
    fn simple() {
        let a = Fate::from_value(3);
        let b = Fate::from_value(5);
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c = Fate::from_expression(
            Box::new(move || a_clone.get() + b_clone.get()),
            vec![a.clone(), b.clone()],
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
            vec![a.clone(), b.clone()],
        );
        assert_eq!(c.get(), 10 + 23 * 23);
        b.bind_value(113);
        assert_eq!(c.get(), 10 + 113 * 113);

        let c_clone = c.clone();
        let a_clone = a.clone();
        let d = Fate::from_expression(
            Box::new(move || c_clone.get() * a_clone.get()),
            vec![c.clone(), a.clone()],
        );

        assert_eq!(d.get(), (10 + 113 * 113) * 10);

        let a_clone = a.clone();
        let b_clone = b.clone();
        let e = Fate::from_value(2);
        let e_clone = e.clone();
        c.bind_expression(
            Box::new(move || a_clone.get() * b_clone.get() / e_clone.get()),
            vec![a.clone(), b.clone(), e.clone()],
        );
        assert_eq!(c.get(), 10 * 113 / 2);
    }

    #[test]
    #[should_panic(expected = "Circular reference")]
    fn circular_reference() {
        let a = Fate::from_value(3);
        let b = Fate::from_value(5);
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c = Fate::from_expression(
            Box::new(move || a_clone.get() + b_clone.get()),
            vec![a.clone(), b.clone()],
        );
        let a_clone = a.clone();
        let c_clone = c.clone();
        b.bind_expression(
            Box::new(move || a_clone.get() + c_clone.get()),
            vec![a.clone(), c.clone()],
        );
    }

    #[test]
    fn macro_simple_test() {
        let a = 5;
        let b = a * 5;
        let c = a * b;
        fate! {
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
}
