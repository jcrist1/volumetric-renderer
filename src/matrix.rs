use std::fmt::Debug;
use std::ops::{Add, Mul};
#[derive(Clone, Debug)]
struct Vector<T, const N: usize> {
    data: [T; N],
}

trait Zero {
    const ZERO: Self;
}

impl<T: Clone, const N: usize> Vector<T, N> {
    fn dot<U, V>(self, rhs: &Vector<U, N>) -> V
    where
        U: Mul<T, Output = V> + Clone,
        V: Add<V, Output = V> + Zero,
    {
        self.data
            .iter()
            .zip(rhs.data.iter())
            .fold(V::ZERO, |accum, (left, right)| {
                accum + (right.clone() * left.clone())
            })
    }
}

impl<T: Clone + Zero, const N: usize> Vector<T, N> {
    fn fit<const M: usize>(&self) -> Vector<T, M> {
        let mut i = 0;
        let data = [(); M].map(|_| {
            let val = if i < N { self.data[i].clone() } else { T::ZERO };
            i += 1;
            val
        });
        Vector { data }
    }
}

impl<T: Clone, U, V, const N: usize> Add<Vector<U, N>> for Vector<T, N>
where
    U: Add<T, Output = V>,
    V: Default + Copy,
{
    type Output = Vector<V, N>;
    fn add(self, rhs: Vector<U, N>) -> Vector<V, N> {
        let mut new_data: [V; N] = [V::default(); N];
        for (target, (left, right)) in new_data
            .iter_mut()
            .zip(self.data.into_iter().zip(rhs.data.into_iter()))
        {
            *target = right + left
        }

        Vector { data: new_data }
    }
}

impl Zero for usize {
    const ZERO: usize = 0;
}

#[derive(Clone)]
struct Matrix<T: Clone, const N: usize, const M: usize> {
    data: [Vector<T, M>; N],
}

impl<T: Clone, const N: usize, const M: usize> Matrix<T, N, M> {
    fn transpose(&self) -> Matrix<T, M, N> {
        let mut i: usize = 0;
        let data: [Vector<T, N>; M] = [0; M].map(|_| {
            let mut j = 0;
            let inner_data: [T; N] = [0; N].map(|_| {
                let val = self.data[j].data[i].clone();
                j += 1;
                val
            });
            let val = Vector { data: inner_data };
            i += 1;
            val
        });
        Matrix { data }
    }
}

impl<T, U, V, const N: usize, const M: usize> Mul<&Vector<U, M>> for Matrix<T, N, M>
where
    T: Clone,
    U: Mul<T, Output = V> + Clone,
    V: Add<V, Output = V> + Zero,
{
    type Output = Vector<V, N>;

    fn mul(self, rhs: &Vector<U, M>) -> Self::Output {
        let data = self.data.clone().map(|left| left.dot(&rhs));
        Vector { data }
    }
}

impl<T, U, V, const N: usize, const M: usize, const P: usize> Mul<&Matrix<U, M, P>>
    for Matrix<T, N, M>
where
    T: Clone,
    U: Mul<T, Output = V> + Clone,
    V: Add<V, Output = V> + Zero + Clone,
{
    type Output = Matrix<V, N, P>;
    fn mul(self, rhs: &Matrix<U, M, P>) -> Matrix<V, N, P> {
        let right_transpose = std::rc::Rc::new(rhs.transpose());
        let data = self.data.map(|left| {
            let mut i: usize = 0;
            let right_transpose_view = &right_transpose.data;
            let data: [V; P] = [0u8; P].map(|_| {
                let right = right_transpose_view[i].clone();
                i += 1;
                left.clone().dot(&right)
            });
            Vector { data }
        });
        Matrix { data }
    }
}

#[cfg(test)]
mod test {
    use super::Vector;
    fn arr_range<const L: usize>(start: &usize) -> Result<[usize; L], String> {
        let mut i = start.clone();
        if (i + L) < i {
            return Err(String::from(
                "Overflow in range start + length must be less that usize limit",
            ));
        }

        Ok([(); L].map(|_| {
            let val = i.clone();
            i += 1;
            val
        }))
    }

    #[test]
    fn test_add() {
        let ten_to_twenty: [usize; 10] = arr_range(&10).unwrap();
        let zero_to_ten: [usize; 10] = arr_range(&0).unwrap();
        let vec: Vector<usize, 10> = Vector { data: zero_to_ten };

        let all_tens = Vector { data: [10; 10] };
        let vec_sum = vec + all_tens;
        assert_eq!(ten_to_twenty, vec_sum.data)
    }

    #[test]
    fn test_dot() {
        const TEN: usize = 23;
        let one_to_10: Vector<usize, TEN> = Vector {
            data: arr_range(&0).unwrap(),
        };
        let const_one = Vector { data: [1; TEN] };
        let v = one_to_10.dot(&const_one);
        assert_eq!(v, TEN * (TEN - 1) / 2)
    }
}
