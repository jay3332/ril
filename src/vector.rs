//! Defines the [`Vector`] type and associated traits for vector operations.
//!
//! This module provides a fixed-size and dynamically-sized vector type that can be used for
//! mathematical operations, pixel manipulation, and other vectorizable tasks.
//!
//! A fixed-size pixel can usually be represented as a vector, as long as it implements
//! [`IntoVector`].

use num_traits::{ConstOne, ConstZero, Num, SaturatingAdd, SaturatingMul, Zero};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, MulAssign, Sub};

/// Vectorizable type trait. This trait is used to indicate that a type (usually a pixel) can
/// be represented as a statically-sized vector.
pub trait IntoVector<const N: usize> {
    /// The type of each element in the vector representation of this type.
    type Element: Sized;

    /// Converts this type into a vector of its elements.
    ///
    /// # Returns
    /// A vector containing the elements of this type.
    #[must_use]
    fn into_vector(self) -> Vector<N, Self::Element>;
}

/// Vectorizable type trait. This trait is used to indicate that a type can be constructed from a
/// vector.
pub trait FromVector<const N: usize> {
    /// The type of each element in the vector representation of this type.
    type Element: Sized;

    /// Constructs a new instance from the given vector.
    #[must_use]
    fn from_vector(vector: Vector<N, Self::Element>) -> Self;
}

/// A mathematical vector represented using a fixed-sized array with elements of type `T`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vector<const N: usize, T>([T; N]);

impl<const N: usize, T> Vector<N, T> {
    /// Creates a new vector with the given elements.
    #[must_use]
    pub const fn new(elements: [T; N]) -> Self {
        Self(elements)
    }

    /// Creates a new vector filled with the given value.
    #[must_use]
    pub const fn of(value: T) -> Self
    where
        T: Copy,
    {
        Self([value; N])
    }

    /// Returns the elements of the vector.
    #[must_use]
    pub const fn elements(&self) -> &[T; N] {
        &self.0
    }

    /// Returns the element at the given index.
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> T
    where
        T: Copy,
    {
        self.0[index]
    }

    /// Performs the element-wise operation `f` on this vector and the given vector `other`.
    #[must_use]
    pub fn merge<U: Copy, R: Copy + Zero>(
        &self,
        other: Vector<N, U>,
        f: impl Fn(T, U) -> R,
    ) -> Vector<N, R>
    where
        T: Copy,
    {
        let mut result = [R::zero(); N];
        for i in 0..N {
            result[i] = f(self[i], other[i]);
        }
        Vector(result)
    }

    /// Performs the operation `f` on each element of this vector.
    #[must_use]
    pub fn map<R: Copy + Zero>(self, f: impl Fn(T) -> R) -> Vector<N, R>
    where
        T: Copy,
    {
        let mut result = [R::zero(); N];
        for i in 0..N {
            result[i] = f(self[i]);
        }
        Vector(result)
    }

    /// Computes the dot product of this vector and the given vector `other`.
    #[must_use]
    pub fn dot<U: Copy, R>(&self, other: Vector<N, U>) -> R
    where
        T: Copy + Mul<U, Output = R>,
        R: Copy + Zero + Add<Output = R>,
    {
        let mut result = R::zero();
        for i in 0..N {
            result = result + (self[i] * other[i]);
        }
        result
    }

    /// Computes the norm (magnitude squared) of this vector.
    #[must_use]
    pub fn norm(&self) -> T
    where
        T: Copy + Zero + Add<Output = T> + Mul<Output = T>,
    {
        self.dot(*self)
    }

    /// Computes the magnitude (length) of this vector.
    #[must_use]
    pub fn magnitude(&self) -> T
    where
        T: Copy + Zero + Add<Output = T> + Mul<Output = T> + num_traits::Float,
    {
        self.norm().sqrt()
    }

    /// Computes the sum of all elements in this vector.
    #[must_use]
    pub fn sum(&self) -> T
    where
        T: Copy + Zero + Add<Output = T>,
    {
        let mut result = T::zero();
        for i in 0..N {
            result = result + self[i];
        }
        result
    }
}

impl<T> Vector<3, T> {
    /// Computes the cross product of this vector and the given vector `other`.
    #[must_use]
    pub fn cross<U: Copy, R>(&self, other: Vector<3, U>) -> Vector<3, R>
    where
        T: Copy + Mul<U, Output = R>,
        R: Copy + Sub<R, Output = R>,
    {
        Vector([
            self[1] * other[2] - self[2] * other[1],
            self[2] * other[0] - self[0] * other[2],
            self[0] * other[1] - self[1] * other[0],
        ])
    }
}

impl<const N: usize, T: Copy + ConstZero> Vector<N, T> {
    /// Creates a new vector with all elements initialized to zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self([T::ZERO; N])
    }
}

impl<const N: usize, T: Copy + ConstOne> Vector<N, T> {
    /// Creates a new vector with all elements initialized to one.
    #[must_use]
    pub const fn one() -> Self {
        Self([T::ONE; N])
    }
}

impl<const N: usize, T> Index<usize> for Vector<N, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const N: usize, T> IndexMut<usize> for Vector<N, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<const N: usize, T: Copy, U: Copy, R: Copy + Zero> Add<Vector<N, U>> for Vector<N, T>
where
    T: Add<U, Output = R>,
{
    type Output = Vector<N, R>;

    fn add(self, other: Vector<N, U>) -> Self::Output {
        self.merge(other, |a, b| a + b)
    }
}

impl<const N: usize, T: Copy> SaturatingAdd for Vector<N, T>
where
    T: SaturatingAdd + Zero,
{
    fn saturating_add(&self, other: &Vector<N, T>) -> Self {
        self.merge(*other, |a, b| a.saturating_add(&b))
    }
}

impl<const N: usize, T: Copy + Zero + Add<Output = T>> AddAssign<Vector<N, T>> for Vector<N, T> {
    fn add_assign(&mut self, other: Vector<N, T>) {
        *self = *self + other;
    }
}

impl<const N: usize, T: Copy, U: Copy, R: Copy + Zero> Sub<Vector<N, U>> for Vector<N, T>
where
    T: Sub<U, Output = R>,
{
    type Output = Vector<N, R>;

    fn sub(self, other: Vector<N, U>) -> Self::Output {
        self.merge(other, |a, b| a - b)
    }
}

impl<const N: usize, T: Copy, U: Copy, R: Copy + Zero> Mul<Vector<N, U>> for Vector<N, T>
where
    T: Mul<U, Output = R>,
{
    type Output = Vector<N, R>;

    fn mul(self, other: Vector<N, U>) -> Self::Output {
        self.merge(other, |a, b| a * b)
    }
}

impl<const N: usize, T: Copy> SaturatingMul for Vector<N, T>
where
    T: SaturatingMul + Zero,
{
    fn saturating_mul(&self, other: &Vector<N, T>) -> Self {
        self.merge(*other, |a, b| a.saturating_mul(&b))
    }
}

impl<const N: usize, T> Mul<T> for Vector<N, T>
where
    T: Copy + Mul<Output = T> + Num,
{
    type Output = Vector<N, T>;

    fn mul(self, scalar: T) -> Self::Output {
        self.map(|x| x * scalar)
    }
}

impl<const N: usize, T> MulAssign<T> for Vector<N, T>
where
    T: Copy + Mul<Output = T> + Num,
{
    fn mul_assign(&mut self, scalar: T) {
        *self = *self * scalar;
    }
}

impl<const N: usize, T: Copy> IntoVector<N> for Vector<N, T> {
    type Element = T;

    fn into_vector(self) -> Vector<N, Self::Element> {
        self
    }
}

impl<const N: usize, T: Copy> FromVector<N> for Vector<N, T> {
    type Element = T;

    fn from_vector(vector: Self) -> Self {
        vector
    }
}
