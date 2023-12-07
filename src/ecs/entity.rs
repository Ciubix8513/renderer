#![allow(dead_code)]

use std::{
    cell::{Ref, RefCell},
    ops::Deref,
};

use rand::Rng;

use super::component::Component;
pub type UUID = u64;

#[derive(Default, Debug)]
pub struct Entity {
    id: UUID,
    components: Vec<std::cell::RefCell<Box<dyn Component + 'static>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ComponentError {
    ComponentDoesNotExist,
    ComponentAlreadyExists,
}

pub struct ComponentGuard<'a, T> {
    guard: Ref<'a, T>,
}
impl<'b, T> Deref for ComponentGuard<'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

pub struct ComponentRefernce<'a, T> {
    phantom: std::marker::PhantomData<T>,
    cell: &'a RefCell<Box<dyn Component + 'static>>,
}
impl<'a, T: 'static> ComponentRefernce<'a, T> {
    fn borrow(&self) -> &'a T {
        let binding = self.cell.borrow();
        binding.as_any().downcast_ref::<T>().unwrap()
    }
}

impl Entity {
    ///Creates a new entity with no components
    pub fn new() -> Self {
        Entity {
            id: rand::thread_rng().gen(),
            components: Vec::new(),
        }
    }

    ///Returns internal entity id
    pub fn get_id(&self) -> UUID {
        self.id
    }

    ///Checks if the entity has component of type T
    pub fn has_component<T: 'static>(&self) -> bool {
        for c in self.components.iter() {
            let any = c.borrow().as_any().downcast_ref::<T>();
            if any.is_some() {
                return true;
            }
        }
        false
    }

    ///Adds component of type T to the entity
    pub fn add_component<T: 'static>(&mut self) -> Result<(), ComponentError>
    where
        T: Component,
    {
        //Check if already have that component
        if self.has_component::<T>() {
            return Err(ComponentError::ComponentAlreadyExists);
        }
        self.components.push(RefCell::new(Box::new(T::mew())));
        self.components.last().unwrap().borrow_mut().awawa();

        Ok(())
    }

    ///Removes component of type T from the entity
    pub fn remove_component<T: 'static>(&mut self) -> Result<(), ComponentError>
    where
        T: Component,
    {
        let mut ind = None;
        for (index, c) in self.components.iter().enumerate() {
            let binding = c.borrow();
            let any = binding.as_any().downcast_ref::<T>();
            if any.is_some() {
                ind = Some(index);
                break;
            }
        }
        if ind.is_none() {
            return Err(ComponentError::ComponentDoesNotExist);
        }

        self.components.remove(ind.unwrap());
        Ok(())
    }

    ///Gets a reference to a component of type T
    pub fn get_component<T: 'static>(&self) -> Result<&RefCell<Box<dyn Component>>, ComponentError>
    where
        T: Component,
    {
        for c in self.components.iter() {
            let binding = c.borrow();
            if binding.as_any().downcast_ref::<T>().is_some() {
                return Ok(c);
            }
        }
        Err(ComponentError::ComponentDoesNotExist)
    }

    ///Performs update on all components of the entity
    pub fn update(&mut self) {
        for c in self.components.iter_mut() {
            c.borrow_mut().update();
        }
    }

    ///Destroys the entity and calls decatification on all of it components
    pub fn decatify(mut self) {
        for c in self.components.iter_mut() {
            c.borrow_mut().decatification();
        }
    }
}

#[cfg(test)]
mod entity_tests {
    use crate::ecs::components::transform::Transform;

    use super::*;

    #[derive(Debug)]
    struct TestComponent {
        pub value: i32,
    }

    impl Component for TestComponent {
        fn mew() -> Self
        where
            Self: Sized,
        {
            Self { value: 0 }
        }

        fn name(&self) -> &'static str {
            "Test entity"
        }

        fn update(&mut self) {
            self.value += 10;
        }

        fn awawa(&mut self) {}
        fn decatification(&mut self) {}

        fn as_any(&self) -> &dyn std::any::Any {
            self as &dyn std::any::Any
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self as &mut dyn std::any::Any
        }
    }

    #[test]
    fn component_add_test() {
        let mut entity = Entity::new();

        assert!(!entity.has_component::<crate::ecs::components::transform::Transform>());

        let res = entity.add_component::<Transform>();

        assert_eq!(res, Ok(()));
        assert!(entity.has_component::<crate::ecs::components::transform::Transform>());
    }

    #[test]
    fn component_remove_test() {
        let mut entity = Entity::new();

        entity.add_component::<Transform>().unwrap();
        let e = entity.remove_component::<Transform>();

        assert_eq!(e, Ok(()));
        assert!(!entity.has_component::<crate::ecs::components::transform::Transform>());
    }

    #[test]
    fn get_component_test() {
        let mut entity = Entity::new();
        entity.add_component::<Transform>().unwrap();
        let c = entity.get_component::<Transform>();
        assert!(c.is_ok());
        entity.remove_component::<Transform>().unwrap();

        let c = entity.get_component::<Transform>();
        assert!(c.is_err());
    }

    #[test]
    fn component_update_test() {
        let mut entity = Entity::new();

        entity.add_component::<TestComponent>().unwrap();
        entity.update();

        // let c = entity
        //     .get_component::<TestComponent>()
        //     .unwrap()
        //     .borrow()
        //     .as_any()
        //     .downcast_ref::<TestComponent>()
        //     .unwrap();
        // assert_eq!(c.value, 10)
    }

    #[test]
    fn component_decatification_test() {
        let mut entity = Entity::new();

        entity.add_component::<TestComponent>().unwrap();
        entity.update();

        // let c = entity.get_component::<TestComponent>().unwrap();
        // assert_eq!(c.value, 10);

        entity.decatify();
    }
}
