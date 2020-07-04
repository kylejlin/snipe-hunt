export type DeepReadonly<T> = T extends Primitive
  ? T
  : T extends (...args: any) => any
  ? DeepReadonlyFunction<T>
  : T extends Array<infer U>
  ? DeepReadonlyArray<U>
  : DeepReadonlyObject<T>;

type Primitive = string | number | boolean | undefined | null;

interface DeepReadonlyFunction<T extends (...args: any) => any> {
  (...args: Parameters<T>): DeepReadonly<ReturnType<T>>;
}

interface DeepReadonlyArray<T> extends ReadonlyArray<DeepReadonly<T>> {}

type DeepReadonlyObject<T> = {
  readonly [P in keyof T]: DeepReadonly<T[P]>;
};
