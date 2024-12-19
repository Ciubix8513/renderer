use std::ops::Div;

///Trait all vectors must implement
pub trait Vector: Div<f32> + Sized + Copy + PartialEq + PartialOrd {
    ///Returns squared length of the vector, much faster than `length()`
    fn square_length(&self) -> f32;
    ///Returns dot product between the `self` vector and the `other` vector
    fn dot_product(&self, other: &Self) -> f32;

    ///Returns length of the vector
    #[must_use]
    fn length(&self) -> f32 {
        self.square_length().sqrt()
    }
    ///Returns vector normalized
    #[must_use]
    fn normalized(&self) -> Self
    where
        Self: From<<Self as Div<f32>>::Output>,
    {
        (*self / self.length()).into()
    }
    ///Returns vector normalized
    #[must_use]
    fn normalize(self) -> Self
    where
        Self: From<<Self as Div<f32>>::Output>,
    {
        let len = self.length();
        if len == 0.0 {
            return self;
        }
        (self / len).into()
    }

    ///Restricts the vector to a certain interval
    ///
    ///Returns max if self is greater than max, and min if self is less than min. Otherwise this returns self.
    fn clamp(self, min: Self, max: Self) -> Self {
        if self > max {
            max
        } else if self < min {
            min
        } else {
            self
        }
    }
}
