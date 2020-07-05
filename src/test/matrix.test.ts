import { Matrix } from "../matrix";

const DECIMALS = 3;

test("Matrix.randomUniform", () => {
  const a = Matrix.randomUniform(6, 1);
  expect([a.rows, a.columns]).toEqual([6, 1]);
  expect(
    Array.from(a.rowMajorOrderEntries()).every(
      (entry) => -1 <= entry && entry < 1
    )
  ).toBe(true);

  const b = Matrix.randomUniform(2, 3);
  expect([b.rows, b.columns]).toEqual([2, 3]);
  expect(
    Array.from(b.rowMajorOrderEntries()).every(
      (entry) => -1 <= entry && entry < 1
    )
  ).toBe(true);
});

test("Matrix.fromEntryInitializer", () => {
  const a = Matrix.fromEntryInitializer(6, 1, () => -4.2);
  expectEquals(
    a,
    Matrix.fromRows([[-4.2], [-4.2], [-4.2], [-4.2], [-4.2], [-4.2]])
  );

  const b = Matrix.fromEntryInitializer(2, 3, () => 9);
  expectEquals(
    b,
    Matrix.fromRows([
      [9, 9, 9],
      [9, 9, 9],
    ])
  );
});

test("Matrix.zeros", () => {
  expect(Matrix.zeros(6, 1).print(DECIMALS)).toMatchSnapshot();
  expect(Matrix.zeros(2, 3).print(DECIMALS)).toMatchSnapshot();
});

test("Matrix.fromRows", () => {
  const a = Matrix.fromRows([
    [1, 2],
    [3, 4],
    [5, 6],
  ]);
  expect(a.print(DECIMALS)).toMatchSnapshot();

  const b = Matrix.fromRows([[7], [8], [9]]);
  expect(b.print(DECIMALS)).toMatchSnapshot();

  const c = Matrix.fromRows([
    [1, -2, 3, 4],
    [-5, -6, 7, -8],
  ]);
  expect(c.print(DECIMALS)).toMatchSnapshot();
});

test("Matrix.fromRows rejects jagged arrays", () => {
  expect(() => {
    Matrix.fromRows([[1, 2], [3], [4, 5]]);
  }).toThrow();
});

test("Matrix.columnVector", () => {
  const a = Matrix.columnVector([1, -2, 3.5]);
  expectEquals(a, Matrix.fromRows([[1], [-2], [3.5]]));
});

test("Matrix.fromRowMajorOrderEntries", () => {
  const a = Matrix.fromRowMajorOrderEntries(3, 2, [1, -2, 3.5, 4.2, 5, 6]);
  expectEquals(
    a,
    Matrix.fromRows([
      [1, -2],
      [3.5, 4.2],
      [5, 6],
    ])
  );

  const b = Matrix.fromRowMajorOrderEntries(2, 3, [1, -2, 3.5, 4.2, 5, 6]);
  expectEquals(
    b,
    Matrix.fromRows([
      [1, -2, 3.5],
      [4.2, 5, 6],
    ])
  );
});

test("Matrix.fromRowMajorOrderEntries throws if number of entries does not match the product of the number rows and columns", () => {
  expect(() => {
    Matrix.fromRowMajorOrderEntries(3, 2, [1, -2, 3.5, 4.2, 5, 6, 7, 8]);
  }).toThrow();
});

test("Matrix.prototype.clone", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const clone = a.clone();

  expectEquals(clone, a);

  // Clones should have the same entries as the original,
  // but they should not reference the same array as the original.
  expect(clone.rowMajorOrderEntries()).toEqual(a.rowMajorOrderEntries());
  expect(clone.rowMajorOrderEntries()).not.toBe(a.rowMajorOrderEntries());
});

test("Matrix.prototype.mutMultiplyScalar", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  a.mutMultiplyScalar(42);

  expectEquals(
    a,
    Matrix.fromRows([
      [-1 * 42, 2 * 42],
      [3 * 42, -4 * 42],
      [-5 * 42, -6 * 42],
    ])
  );

  const b = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  b.mutMultiplyScalar(-0.3);

  expectEquals(
    b,
    Matrix.fromRows([
      [-1 * -0.3, 2 * -0.3],
      [3 * -0.3, -4 * -0.3],
      [-5 * -0.3, -6 * -0.3],
    ])
  );
});

test("Matrix.prototype.multiplyScalarInto", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = a.multiplyScalarInto(-7.3, Matrix.zeros(a.rows, a.columns));
  const c = a.multiplyScalarInto(-7.3, a);
  const expected = Matrix.fromRows([
    [-1 * -7.3, 2 * -7.3],
    [3 * -7.3, -4 * -7.3],
    [-5 * -7.3, -6 * -7.3],
  ]);

  expectEquals(b, expected);
  expectEquals(c, expected);
  expect(c).toBe(a);
});

test("Matrix.prototype.multiplyScalarInto throws if out matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);

  expect(() => {
    a.multiplyScalarInto(-7.3, Matrix.zeros(a.rows, a.rows));
  }).toThrow();

  expect(() => {
    a.multiplyScalarInto(-7.3, Matrix.zeros(a.columns, a.columns));
  }).toThrow();

  expect(() => {
    a.multiplyScalarInto(-7.3, Matrix.zeros(a.columns, a.rows));
  }).toThrow();
});

test("Matrix.prototype.mutAdd", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  a.mutAdd(
    Matrix.fromRows([
      [7.3, -8],
      [-9, 10.5],
      [-11, 12],
    ])
  );

  expectEquals(
    a,
    Matrix.fromRows([
      [-1 + 7.3, 2 + -8],
      [3 + -9, -4 + 10.5],
      [-5 + -11, -6 + 12],
    ])
  );
});

test("Matrix.prototye.mutAdd throws if RHS matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-1, 2, 3],
    [-4, 5, 6],
  ]);

  expect(() => {
    a.mutAdd(b);
  }).toThrow();
});

test("Matrix.prototype.mutSubtract", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  a.mutSubtract(
    Matrix.fromRows([
      [7.3, -8],
      [-9, 10.5],
      [-11, 12],
    ])
  );

  expectEquals(
    a,
    Matrix.fromRows([
      [-1 - 7.3, 2 - -8],
      [3 - -9, -4 - 10.5],
      [-5 - -11, -6 - 12],
    ])
  );
});

test("Matrix.prototype.mutSubtract throws if RHS matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-1, 2, 3],
    [-4, 5, 6],
  ]);

  expect(() => {
    a.mutSubtract(b);
  }).toThrow();
});

test("Matrix.prototype.immutSubtract", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = a.immutSubtract(
    Matrix.fromRows([
      [7.3, -8],
      [-9, 10.5],
      [-11, 12],
    ])
  );

  expectEquals(
    b,
    Matrix.fromRows([
      [-1 - 7.3, 2 - -8],
      [3 - -9, -4 - 10.5],
      [-5 - -11, -6 - 12],
    ])
  );
});

test("Matrix.prototype.immutSubtract throws if RHS matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-1, 2, 3],
    [-4, 5, 6],
  ]);

  expect(() => {
    a.immutSubtract(b);
  }).toThrow();
});

test("Matrix.prototype.subtractInto", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const subtrahend = Matrix.fromRows([
    [7.3, -8],
    [-9, 10.5],
    [-11, 12],
  ]);
  const b = a.subtractInto(subtrahend, Matrix.zeros(a.rows, a.columns));
  const c = a.subtractInto(subtrahend, a);
  const expected = Matrix.fromRows([
    [-1 - 7.3, 2 - -8],
    [3 - -9, -4 - 10.5],
    [-5 - -11, -6 - 12],
  ]);

  expectEquals(b, expected);
  expectEquals(c, expected);
  expect(c).toBe(a);
});

test("Matrix.prototype.subtractInto throws if RHS matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-1, 2, 3],
    [-4, 5, 6],
  ]);

  expect(() => {
    a.subtractInto(b, Matrix.zeros(a.rows, a.columns));
  }).toThrow();
});

test("Matrix.prototype.subtractInto throws if out matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-1, 2, 3],
    [-4, 5, 6],
  ]);

  expect(() => {
    a.subtractInto(a, b);
  }).toThrow();
});

test("Matrix.prototype.subtractScalarInto", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = a.subtractScalarInto(-2.3, Matrix.zeros(a.rows, a.columns));
  const c = a.subtractScalarInto(-2.3, a);
  const expected = Matrix.fromRows([
    [-1 - -2.3, 2 - -2.3],
    [3 - -2.3, -4 - -2.3],
    [-5 - -2.3, -6 - -2.3],
  ]);

  expectEquals(b, expected);
  expectEquals(c, expected);
  expect(c).toBe(a);
});

test("Matrix.prototype.subtractScalarInto throws if out matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);

  expect(() => {
    a.subtractScalarInto(-2.3, Matrix.zeros(a.rows, a.rows));
  });
  expect(() => {
    a.subtractScalarInto(-2.3, Matrix.zeros(a.columns, a.columns));
  });
  expect(() => {
    a.subtractScalarInto(-2.3, Matrix.zeros(a.columns, a.rows));
  });
});

test("Matrix.prototype.immutMultiply", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
  ]);

  expectEquals(
    a.immutMultiply(b),
    Matrix.fromRows([
      [-13, 14, 15],
      [19, -20, -21],
      [95, -106, -117],
    ])
  );

  expectEquals(
    b.immutMultiply(a),
    Matrix.fromRows([
      [-14, -100],
      [-17, -136],
    ])
  );
});

test("Matrix.prototype.immutMultiply throws if this.columns !== RHS.rows", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);

  expect(() => {
    a.immutMultiply(a);
  }).toThrow();

  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
    [-13, 14, 15],
  ]);

  expect(() => {
    a.immutMultiply(b);
  }).toThrow();
});

test("Matrix.prototype.multiplyInto", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
  ]);
  const c = a.multiplyInto(b, Matrix.zeros(a.rows, b.columns));
  const d = b.multiplyInto(a, Matrix.zeros(b.rows, a.columns));

  expectEquals(
    c,
    Matrix.fromRows([
      [-13, 14, 15],
      [19, -20, -21],
      [95, -106, -117],
    ])
  );

  expectEquals(
    d,
    Matrix.fromRows([
      [-14, -100],
      [-17, -136],
    ])
  );
});

test("Matrix.prototype.multiplyInto throws if this.columns !== RHS.rows", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);

  expect(() => {
    a.multiplyInto(a, Matrix.zeros(a.rows, a.columns));
  }).toThrow();

  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
    [-13, 14, 15],
  ]);

  expect(() => {
    a.multiplyInto(b, Matrix.zeros(a.rows, b.columns));
  }).toThrow();
});

test("Matrix.prototype.multiplyInto throws if either this.rows !== out.rows or other.columns !== out.columns", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
  ]);

  expect(() => {
    a.multiplyInto(b, Matrix.zeros(a.rows, a.columns));
  }).toThrow();

  expect(() => {
    a.multiplyInto(b, Matrix.zeros(b.rows, b.columns));
  }).toThrow();

  expect(() => {
    a.multiplyInto(b, Matrix.zeros(b.rows, a.columns));
  }).toThrow();

  expect(() => {
    b.multiplyInto(a, Matrix.zeros(a.rows, a.columns));
  }).toThrow();

  expect(() => {
    b.multiplyInto(a, Matrix.zeros(b.rows, b.columns));
  }).toThrow();

  expect(() => {
    b.multiplyInto(a, Matrix.zeros(a.rows, b.columns));
  }).toThrow();
});

test("Matrix.prototype.mutHadamard", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  a.mutHadamard(
    Matrix.fromRows([
      [-7.1, 8],
      [9, -10.2],
      [11.9, 12],
    ])
  );

  expectEquals(
    a,
    Matrix.fromRows([
      [-1 * -7.1, 2 * 8],
      [3 * 9, -4 * -10.2],
      [-5 * 11.9, -6 * 12],
    ])
  );
});

test("Matrix.prototype.mutHadamard throws if RHS matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = Matrix.fromRows([
    [-7, 8, 9],
    [-10, 11, 12],
  ]);

  expect(() => {
    a.mutHadamard(b);
  }).toThrow();

  expect(() => {
    b.mutHadamard(a);
  }).toThrow();
});

test("Matrix.prototype.immutTranspose", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const aTrans = a.immutTranspose();
  expectEquals(
    aTrans,
    Matrix.fromRows([
      [-1, 3, -5],
      [2, -4, -6],
    ])
  );
});

test("Matrix.prototype.transposeInto", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const aTrans = a.transposeInto(Matrix.zeros(a.columns, a.rows));
  expectEquals(
    aTrans,
    Matrix.fromRows([
      [-1, 3, -5],
      [2, -4, -6],
    ])
  );
});

test("Matrix.prototype.transposeInto throws if either this.rows !== out.columns or this.columns !== out.rows", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);

  expect(() => {
    a.transposeInto(Matrix.zeros(a.rows, a.columns));
  }).toThrow();
});

test("Matrix.prototype.rowMajorOrderEntries", () => {
  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  expect(a.rowMajorOrderEntries()).toEqual(
    Float64Array.from([-1, 2, 3, -4, -5, -6])
  );
});

test("Matrix.prototype.immutApplyElementwise", () => {
  function cube(x: number): number {
    return Math.pow(x, 3);
  }

  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  expectEquals(
    a.immutApplyElementwise(cube),
    Matrix.fromRows([
      [cube(-1), cube(2)],
      [cube(3), cube(-4)],
      [cube(-5), cube(-6)],
    ])
  );
});

test("Matrix.prototype.applyElementwiseInto", () => {
  function cube(x: number): number {
    return Math.pow(x, 3);
  }

  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = a.applyElementwiseInto(cube, Matrix.zeros(a.rows, a.columns));
  const c = a.applyElementwiseInto(cube, a);
  const expected = Matrix.fromRows([
    [cube(-1), cube(2)],
    [cube(3), cube(-4)],
    [cube(-5), cube(-6)],
  ]);

  expectEquals(b, expected);

  expectEquals(c, expected);

  expect(c).toBe(a);
});

test("Matrix.prototype.applyElementwiseInto throws if the out matrix has different dimensions", () => {
  function cube(x: number): number {
    return Math.pow(x, 3);
  }

  const a = Matrix.fromRows([
    [-1, 2],
    [3, -4],
    [-5, -6],
  ]);
  const b = a.immutTranspose();

  expect(() => {
    a.applyElementwiseInto(cube, b);
  }).toThrow();
});

test("Matrix.prototype.copyInto", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [3.5, 4.2],
    [5, 6],
  ]);
  const b = a.copyInto(Matrix.zeros(a.rows, a.columns));
  expectEquals(a, b);
});

test("Matrix.prototype.copyInto throws if out matrix has different dimensions", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [3.5, 4.2],
    [5, 6],
  ]);

  expect(() => {
    a.copyInto(Matrix.zeros(a.rows, a.rows));
  }).toThrow();

  expect(() => {
    a.copyInto(Matrix.zeros(a.columns, a.columns));
  }).toThrow();

  expect(() => {
    a.copyInto(Matrix.zeros(a.columns, a.rows));
  }).toThrow();
});

test("Matrix.prototype.setToZero", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [3.5, 4.2],
    [5, 6],
  ]);
  a.setToZero();

  expectEquals(a, Matrix.zeros(3, 2));
});

test("Matrix.prototype.maxEntryExcludingLast()", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [-3.5, 4.2],
    [5, 6.1],
  ]);
  const b = Matrix.fromRows([
    [-11, 12.6],
    [-10.5, 9.2],
    [8, 7.1],
  ]);

  expect(a.maxEntryExcludingLast()).toBe(5);
  expect(b.maxEntryExcludingLast()).toBe(12.6);
});

test("Matrix.prototype.sumOfAllEntriesButLast()", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [-3.5, 4.2],
    [5, 6.1],
  ]);
  const b = Matrix.fromRows([
    [-11, 12.6],
    [-10.5, 9.2],
    [8, 7.1],
  ]);

  expect(a.sumOfAllEntriesButLast()).toBe(1 + -2 + -3.5 + 4.2 + 5);
  expect(b.sumOfAllEntriesButLast()).toBe(-11 + 12.6 + -10.5 + 9.2 + 8);
});

test("Matrix.prototype.mutFilterAllButLast", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [-3.5, 4.2],
    [5, 6.1],
  ]).mutFilterAllButLast([0, 1, 1, 0, 0]);
  expectEquals(
    a,
    Matrix.fromRows([
      [0, -2],
      [-3.5, 0],
      [0, 6.1],
    ])
  );
});

test("Matrix.prototype.setLastEntry", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [-3.5, 4.2],
    [5, 6.1],
  ]);
  a.setLastEntry(-7.3);
  expectEquals(
    a,
    Matrix.fromRows([
      [1, -2],
      [-3.5, 4.2],
      [5, -7.3],
    ])
  );
});

test("Matrix.prototype.lastEntry", () => {
  const a = Matrix.fromRows([
    [1, -2],
    [-3.5, 4.2],
    [5, 6.1],
  ]);
  expect(a.lastEntry()).toBe(6.1);
});

function expectEquals(a: Matrix, b: Matrix): void {
  expect(a.print(DECIMALS)).toBe(b.print(DECIMALS));
}
