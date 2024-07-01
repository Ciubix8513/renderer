use crate::math::{mat4x4::Mat4x4, vec3::Vec3};

use crate::ecs::{Component, ComponentReference};

///Transform  component contains function and data to determine the position of the entity
///
///Note: rotation is represented as Euler angles using degrees
#[derive(Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    parent: Option<ComponentReference<Self>>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            rotation: Vec3::default(),
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            parent: None,
        }
    }
}

impl Component for Transform {
    fn mew() -> Self
    where
        Self: Sized,
    {
        Self {
            rotation: Vec3::default(),
            scale: Vec3::new(1.0, 1.0, 1.0),
            position: Vec3::default(),
            parent: None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

impl Transform {
    ///Create a new transform instance
    #[must_use]
    pub const fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
            parent: None,
        }
    }

    ///Creates a new transform instance, with a parent
    pub fn with_parent(
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
        parent: ComponentReference<Transform>,
    ) -> Self {
        Self {
            position,
            rotation,
            scale,
            parent: Some(parent),
        }
    }

    ///Returns transformation of the entity taking transform of the parent into account
    #[must_use]
    pub fn matrix(&self) -> Mat4x4 {
        if let Some(p) = &self.parent {
            let parent_mat = p.borrow().matrix();
            parent_mat * Mat4x4::transform_matrix_euler(&self.position, &self.scale, &self.rotation)
        } else {
            Mat4x4::transform_matrix_euler(&self.position, &self.scale, &self.rotation)
        }
    }

    //Returns transformation matrix of the entity, without taking the parent transformation into
    //account
    #[must_use]
    pub fn matrix_local(&self) -> Mat4x4 {
        Mat4x4::transform_matrix_euler(&self.position, &self.scale, &self.rotation)
    }

    ///Sets the parent of the entity, applying all parent transformations to this entity
    pub fn set_parent(mut self, p: ComponentReference<Transform>) {
        self.parent = Some(p);
    }
}
