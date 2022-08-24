use alexandria_common::{Matrix, Vector3, Vector4};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LHRowMajorMatrix([f32; 4 * 4]);

impl Matrix for LHRowMajorMatrix {
    fn zero() -> Self {
        LHRowMajorMatrix([0.0; 4 * 4])
    }

    fn identity() -> Self {
        let mut matrix = LHRowMajorMatrix::zero();
        matrix.set(0, 0, 1.0);
        matrix.set(1, 1, 1.0);
        matrix.set(2, 2, 1.0);
        matrix.set(3, 3, 1.0);
        matrix
    }

    fn look_at(position: Vector3, target: Vector3, up: Vector3) -> LHRowMajorMatrix {
        let z_axis = (target - position).normal();
        let x_axis = (up.cross(z_axis)).normal();
        let y_axis = z_axis.cross(x_axis);

        let mut matrix = LHRowMajorMatrix::zero();
        matrix.set(0, 0, x_axis.x());
        matrix.set(0, 1, x_axis.y());
        matrix.set(0, 2, x_axis.z());
        matrix.set(0, 3, -x_axis.dot(position));
        matrix.set(1, 0, y_axis.x());
        matrix.set(1, 1, y_axis.y());
        matrix.set(1, 2, y_axis.z());
        matrix.set(1, 3, -y_axis.dot(position));
        matrix.set(2, 0, z_axis.x());
        matrix.set(2, 1, z_axis.y());
        matrix.set(2, 2, z_axis.z());
        matrix.set(2, 3, -z_axis.dot(position));
        matrix.set(3, 3, 1.0);
        matrix
    }

    fn scale(x: f32, y: f32, z: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();
        matrix.set(0, 0, x);
        matrix.set(1, 1, y);
        matrix.set(2, 2, z);
        matrix
    }

    fn translation(x: f32, y: f32, z: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();
        matrix.set(0, 3, x);
        matrix.set(1, 3, y);
        matrix.set(2, 3, z);
        matrix
    }

    fn rotation(x: f32, y: f32, z: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();

        let cos_a = z.cos();
        let sin_a = z.sin();
        let cos_b = y.cos();
        let sin_b = y.sin();
        let cos_g = x.cos();
        let sin_g = x.sin();

        matrix.set(0, 0, cos_a * cos_b);
        matrix.set(0, 1, sin_a * cos_b);
        matrix.set(0, 2, -sin_b);

        matrix.set(1, 0, cos_a * sin_b * sin_g - sin_a * cos_g);
        matrix.set(1, 1, sin_a * sin_b * sin_g + cos_a * cos_g);
        matrix.set(1, 2, cos_b * sin_g);

        matrix.set(2, 0, cos_a * sin_b * cos_g + sin_a * sin_g);
        matrix.set(2, 1, sin_a * sin_b * cos_g - cos_a * sin_g);
        matrix.set(2, 2, cos_b * cos_g);

        matrix
    }

    fn rotation_x(angle: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();

        let c = angle.cos();
        let s = angle.sin();

        matrix.set(1, 1, c);
        matrix.set(2, 1, -s);
        matrix.set(1, 2, s);
        matrix.set(2, 2, c);

        matrix
    }

    fn rotation_y(angle: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();

        let c = angle.cos();
        let s = angle.sin();

        matrix.set(0, 0, c);
        matrix.set(2, 0, s);
        matrix.set(0, 2, -s);
        matrix.set(2, 2, c);

        matrix
    }

    fn rotation_z(angle: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();

        let c = angle.cos();
        let s = angle.sin();

        matrix.set(0, 0, c);
        matrix.set(1, 0, -s);
        matrix.set(0, 1, s);
        matrix.set(1, 1, c);

        matrix
    }

    fn orthographic(width: f32, height: f32, near: f32, far: f32) -> LHRowMajorMatrix {
        let mut matrix = LHRowMajorMatrix::identity();
        matrix.set(0, 0, 2.0 / width);
        matrix.set(1, 1, 2.0 / height);
        matrix.set(2, 2, 1.0 / (far - near));
        matrix.set(2, 3, -near / (far - near));
        matrix
    }

    fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> LHRowMajorMatrix {
        let y_scale = 1.0 / (fovy / 2.0).tan();
        let x_scale = y_scale / aspect;

        let mut matrix = LHRowMajorMatrix::zero();
        matrix.set(0, 0, x_scale);
        matrix.set(1, 1, y_scale);
        matrix.set(2, 2, far / (far - near));
        matrix.set(2, 3, -(near * far) / (far - near));
        matrix.set(3, 2, 1.0);
        matrix
    }

    fn get(&self, col: usize, row: usize) -> f32 {
        self.0[col * 4 + row]
    }

    fn set(&mut self, col: usize, row: usize, val: f32) {
        self.0[col * 4 + row] = val
    }
}

impl Add for LHRowMajorMatrix {
    type Output = LHRowMajorMatrix;

    fn add(mut self, rhs: LHRowMajorMatrix) -> LHRowMajorMatrix {
        for i in 0..4 {
            for j in 0..4 {
                self.set(i, j, self.get(i, j) + rhs.get(i, j))
            }
        }

        self
    }
}

impl AddAssign for LHRowMajorMatrix {
    fn add_assign(&mut self, rhs: LHRowMajorMatrix) {
        *self = *self + rhs;
    }
}

impl Sub for LHRowMajorMatrix {
    type Output = LHRowMajorMatrix;

    fn sub(mut self, rhs: LHRowMajorMatrix) -> LHRowMajorMatrix {
        for i in 0..4 {
            for j in 0..4 {
                self.set(i, j, self.get(i, j) - rhs.get(i, j))
            }
        }

        self
    }
}

impl SubAssign for LHRowMajorMatrix {
    fn sub_assign(&mut self, rhs: LHRowMajorMatrix) {
        *self = *self - rhs;
    }
}

impl Mul<Vector4> for LHRowMajorMatrix {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Vector4 {
        Vector4::new(
            self.get(0, 0) * rhs.x()
                + self.get(1, 0) * rhs.y()
                + self.get(2, 0) * rhs.z()
                + self.get(3, 0) * rhs.w(),
            self.get(0, 1) * rhs.x()
                + self.get(1, 1) * rhs.y()
                + self.get(2, 1) * rhs.z()
                + self.get(3, 1) * rhs.w(),
            self.get(0, 2) * rhs.x()
                + self.get(1, 2) * rhs.y()
                + self.get(2, 2) * rhs.z()
                + self.get(2, 3) * rhs.w(),
            self.get(0, 3) * rhs.x()
                + self.get(1, 3) * rhs.y()
                + self.get(2, 3) * rhs.z()
                + self.get(3, 3) * rhs.w(),
        )
    }
}

impl Mul for LHRowMajorMatrix {
    type Output = LHRowMajorMatrix;

    fn mul(self, rhs: LHRowMajorMatrix) -> LHRowMajorMatrix {
        let mut ret = LHRowMajorMatrix::zero();

        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    ret.set(i, j, ret.get(i, j) + self.get(i, k) * rhs.get(k, j));
                }
            }
        }

        ret
    }
}

impl MulAssign for LHRowMajorMatrix {
    fn mul_assign(&mut self, rhs: LHRowMajorMatrix) {
        *self = *self * rhs;
    }
}

impl From<[f32; 4 * 4]> for LHRowMajorMatrix {
    fn from(vals: [f32; 4 * 4]) -> LHRowMajorMatrix {
        LHRowMajorMatrix(vals)
    }
}

impl Into<[f32; 4 * 4]> for LHRowMajorMatrix {
    fn into(self) -> [f32; 4 * 4] {
        self.0
    }
}

impl Index<(usize, usize)> for LHRowMajorMatrix {
    type Output = f32;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.0[index.0 + index.1 * 4]
    }
}

impl IndexMut<(usize, usize)> for LHRowMajorMatrix {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.0[index.0 + index.1 * 4]
    }
}

impl std::fmt::Display for LHRowMajorMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for i in 0..4 {
            writeln!(
                f,
                "| {: <5} {: <5} {: <5} {: <5} |",
                self.0[i * 4 + 0],
                self.0[i * 4 + 1],
                self.0[i * 4 + 2],
                self.0[i * 4 + 3]
            )?;
        }

        Ok(())
    }
}
