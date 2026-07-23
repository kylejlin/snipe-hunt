export class Matrix {
  /** @deprecated Use `Matrix.fromEntryInitializer(rows, columns, uniformRng)` instead. */
  static randomUniform(rows: number, columns: number): Matrix {
    const size = rows * columns;
    const data = new Float64Array(size);
    for (let i = 0; i < size; i++) {
      data[i] = Math.random() * 2 - 1;
    }
    return new Matrix(rows, columns, data);
  }

  static fromEntryInitializer(
    rows: number,
    columns: number,
    initializer: () => number
  ) {
    const data = new Float64Array(rows * columns).map(initializer);
    return new Matrix(rows, columns, data);
  }

  static zeros(rows: number, columns: number): Matrix {
    const data = new Float64Array(rows * columns);
    return new Matrix(rows, columns, data);
  }

  static fromRows(rows: number[][]): Matrix {
    const columns = rows[0].length;
    if (rows.some((row) => row.length !== columns)) {
      throw new Error(
        "Cannot create a matrix from a jagged array: " + JSON.stringify(rows)
      );
    }

    return new Matrix(rows.length, columns, rows.flat());
  }

  static columnVector(entries: number[]): Matrix {
    return new Matrix(entries.length, 1, entries);
  }

  static fromRowMajorOrderEntries(
    rows: number,
    columns: number,
    entries: ArrayLike<number>
  ): Matrix {
    if (entries.length !== rows * columns) {
      throw new Error(
        "Expected " +
          rows * columns +
          " entries but instead got " +
          entries.length +
          "."
      );
    }

    return new Matrix(rows, columns, entries);
  }

  public readonly rows: number;
  public readonly columns: number;
  private data: Float64Array;

  private constructor(rows: number, columns: number, data: ArrayLike<number>) {
    this.rows = rows;
    this.columns = columns;
    this.data = data instanceof Float64Array ? data : Float64Array.from(data);
  }

  clone(): Matrix {
    return new Matrix(this.rows, this.columns, this.data.slice());
  }

  mutMultiplyScalar(n: number): this {
    const size = this.data.length;
    for (let i = 0; i < size; i++) {
      this.data[i] *= n;
    }
    return this;
  }

  multiplyScalarInto(n: number, out: Matrix): Matrix {
    if (!(this.rows === out.rows && this.columns === out.columns)) {
      throw new Error(
        "Cannot multiply a scalar " +
          n +
          " by a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix. The out matrix must have the same dimensions as this matrix."
      );
    }

    const thisData = this.data;
    const outData = out.data;
    const outSize = outData.length;
    for (let i = 0; i < outSize; i++) {
      outData[i] = n * thisData[i];
    }
    return out;
  }

  mutAdd(other: Matrix): this {
    if (!(other.rows === this.rows && other.columns === this.columns)) {
      throw new TypeError(
        "Cannot add a " +
          this.rows +
          "x" +
          this.columns +
          " to a " +
          other.rows +
          "x" +
          other.columns +
          " matrix."
      );
    }

    const size = this.data.length;
    for (let i = 0; i < size; i++) {
      this.data[i] += other.data[i];
    }

    return this;
  }

  mutSubtract(other: Matrix): this {
    if (!(other.rows === this.rows && other.columns === this.columns)) {
      throw new TypeError(
        "Cannot add a " +
          this.rows +
          "x" +
          this.columns +
          " to a " +
          other.rows +
          "x" +
          other.columns +
          " matrix."
      );
    }

    const size = this.data.length;
    for (let i = 0; i < size; i++) {
      this.data[i] -= other.data[i];
    }

    return this;
  }

  immutSubtract(other: Matrix): Matrix {
    return this.subtractInto(other, this.clone());
  }

  subtractInto(other: Matrix, out: Matrix): Matrix {
    if (!(other.rows === this.rows && other.columns === this.columns)) {
      throw new TypeError(
        "Cannot subtract a " +
          other.rows +
          "x" +
          other.columns +
          " matrix from a " +
          this.rows +
          "x" +
          this.columns +
          " matrix."
      );
    }

    if (!(this.rows === out.rows && this.columns === out.columns)) {
      throw new TypeError(
        "Cannot subtract a " +
          other.rows +
          "x" +
          other.columns +
          " matrix from a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix. The source matrix and destination matrix must have the same dimensions."
      );
    }

    const thisData = this.data;
    const otherData = other.data;
    const outData = out.data;
    const outSize = outData.length;
    for (let i = 0; i < outSize; i++) {
      outData[i] = thisData[i] - otherData[i];
    }
    return out;
  }

  subtractScalarInto(n: number, out: Matrix): Matrix {
    if (!(this.rows === out.rows && this.columns === out.columns)) {
      throw new TypeError(
        "Cannot subtract a scalar from a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix. MThe source matrix and destination matrix must have the same dimensions."
      );
    }

    const thisData = this.data;
    const outData = out.data;
    const outSize = outData.length;
    for (let i = 0; i < outSize; i++) {
      outData[i] = thisData[i] - n;
    }
    return out;
  }

  immutMultiply(other: Matrix): Matrix {
    return this.multiplyInto(other, Matrix.zeros(this.rows, other.columns));
  }

  multiplyInto(other: Matrix, out: Matrix): Matrix {
    if (this.columns !== other.rows) {
      throw new TypeError(
        "Cannot multiply a " +
          this.rows +
          "x" +
          this.columns +
          " matrix with a " +
          other.rows +
          "x" +
          other.columns +
          " matrix."
      );
    }

    if (!(this.rows === out.rows && other.columns === out.columns)) {
      throw new TypeError(
        "Cannot multiply a " +
          this.rows +
          "x" +
          this.columns +
          " matrix with a " +
          other.rows +
          "x" +
          other.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix."
      );
    }

    const thisData = this.data;
    const otherData = other.data;
    const outData = out.data;
    const thisRows = this.rows;
    const otherColumns = other.columns;
    const thisColumns = this.columns;
    const outColumns = out.columns;

    for (let thisR = 0; thisR < thisRows; thisR++) {
      for (let otherC = 0; otherC < otherColumns; otherC++) {
        let dot = 0;
        for (let thisC = 0; thisC < thisColumns; thisC++) {
          dot +=
            thisData[thisR * thisColumns + thisC] *
            otherData[thisC * otherColumns + otherC];
        }
        outData[thisR * outColumns + otherC] = dot;
      }
    }
    return out;
  }

  mutHadamard(other: Matrix): this {
    if (!(other.rows === this.rows && other.columns === this.columns)) {
      throw new TypeError(
        "Cannot take the Hadamard product of a " +
          this.rows +
          "x" +
          this.columns +
          " matrix and a " +
          other.rows +
          "x" +
          other.columns +
          " matrix."
      );
    }

    const size = this.data.length;
    for (let i = 0; i < size; i++) {
      this.data[i] *= other.data[i];
    }
    return this;
  }

  immutTranspose(): Matrix {
    return this.transposeInto(
      new Matrix(this.columns, this.rows, new Float64Array(this.data.length))
    );
  }

  transposeInto(out: Matrix): Matrix {
    if (!(this.rows === out.columns && this.columns === out.rows)) {
      throw new Error(
        "Cannot transpose a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix."
      );
    }

    const thisData = this.data;
    const thisRows = this.rows;
    const thisColumns = this.columns;
    const outData = out.data;
    const outColumns = out.columns;

    for (let r = 0; r < thisRows; r++) {
      for (let c = 0; c < thisColumns; c++) {
        outData[c * outColumns + r] = thisData[r * thisColumns + c];
      }
    }
    return out;
  }

  rowMajorOrderEntries(): ArrayLike<number> {
    return this.data;
  }

  immutApplyElementwise(f: (entry: number) => number): Matrix {
    return this.applyElementwiseInto(f, this.clone());
  }

  applyElementwiseInto(f: (entry: number) => number, out: Matrix): Matrix {
    if (!(this.rows === out.rows && this.columns === out.columns)) {
      throw new TypeError(
        "Cannot apply " +
          f.name +
          " elementwise on a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix. Matrices must have the same dimensions."
      );
    }

    const thisData = this.data;
    const outData = out.data;
    const outSize = outData.length;
    for (let i = 0; i < outSize; i++) {
      outData[i] = f(thisData[i]);
    }
    return out;
  }

  copyInto(out: Matrix): Matrix {
    if (!(this.rows === out.rows && this.columns === out.columns)) {
      throw new Error(
        "Cannot copy a " +
          this.rows +
          "x" +
          this.columns +
          " matrix into a " +
          out.rows +
          "x" +
          out.columns +
          " matrix."
      );
    }

    const thisData = this.data;
    const outData = out.data;
    const outSize = outData.length;
    for (let i = 0; i < outSize; i++) {
      outData[i] = thisData[i];
    }
    return out;
  }

  setToZero(): void {
    const thisData = this.data;
    const thisSize = thisData.length;
    for (let i = 0; i < thisSize; i++) {
      thisData[i] = 0;
    }
  }

  maxEntryExcludingLast(): number {
    let max = -Infinity;
    const thisData = this.data;
    const thisSizeMinus1 = thisData.length - 1;
    for (let i = 0; i < thisSizeMinus1; i++) {
      const v = thisData[i];
      if (v > max) {
        max = v;
      }
    }
    return max;
  }

  sumOfAllEntriesButLast(): number {
    let sum = 0;
    const thisData = this.data;
    const thisSizeMinus1 = thisData.length - 1;
    for (let i = 0; i < thisSizeMinus1; i++) {
      sum += thisData[i];
    }
    return sum;
  }

  mutFilterAllButLast(filter: ArrayLike<number | boolean>): this {
    const thisData = this.data;
    const thisSizeMinus1 = thisData.length - 1;
    for (let i = 0; i < thisSizeMinus1; i++) {
      if (!filter[i]) {
        thisData[i] = 0;
      }
    }
    return this;
  }

  setLastEntry(entry: number | boolean): void {
    const thisData = this.data;
    // Booleans will get automatically coerced to numbers
    thisData[thisData.length - 1] = entry as number;
  }

  lastEntry(): number {
    const thisData = this.data;
    return thisData[thisData.length - 1];
  }

  print(decimals: number): string {
    const entries = Array.from(this.rowMajorOrderEntries());
    const entryStrings = entries.map((entry) => entry.toFixed(decimals));
    const entryStringLengths = entryStrings.map((s) => s.length);
    const maxLength = Math.max(...entryStringLengths);

    const topAndBottomBorder = "-".repeat(
      this.columns * (maxLength + " | ".length) - " | ".length
    );

    let str = topAndBottomBorder + "\n";

    for (let r = 0; r < this.rows; r++) {
      for (let c = 0; c < this.columns; c++) {
        str +=
          leftpad(entryStrings[r * this.columns + c], maxLength, " ") + " | ";
      }

      str = str.slice(0, -" | ".length);

      str += "\n";
    }

    str += topAndBottomBorder;
    return str;
  }
}

function leftpad(s: string, minLength: number, fillCharacter: string): string {
  const diff = minLength - s.length;
  if (diff <= 0) {
    return s;
  }

  return fillCharacter.repeat(diff) + s;
}
