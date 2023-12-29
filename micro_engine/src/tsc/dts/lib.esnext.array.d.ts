/*! *****************************************************************************
Copyright (c) Microsoft Corporation. All rights reserved.
Licensed under the Apache License, Version 2.0 (the "License"); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0

THIS CODE IS PROVIDED ON AN *AS IS* BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
KIND, EITHER EXPRESS OR IMPLIED, INCLUDING WITHOUT LIMITATION ANY IMPLIED
WARRANTIES OR CONDITIONS OF TITLE, FITNESS FOR A PARTICULAR PURPOSE,
MERCHANTABLITY OR NON-INFRINGEMENT.

See the Apache Version 2.0 License for specific language governing permissions
and limitations under the License.
***************************************************************************** */

/// <reference no-default-lib="true"/>

interface Array<T> {
  /** Copies and reverses the elements in an array.*/
  toReversed(): T[];

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if the first argument is less than the second argument, zero if they're equal, and a positive
   * value otherwise. If omitted, the elements are sorted in ascending, ASCII character order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: T, b: T) => number): T[];

  /**
   * Copies and elements from an array and, if necessary, inserts new elements in their place, returning the deleted elements.
   * @param start The zero-based location in the array from which to start removing elements.
   * @param deleteCount The number of elements to remove.
   * @returns An array containing the elements that were deleted.
   */
  toSpliced(start: number, deleteCount?: number): T[];
  /**
   * Copies and removes elements from an array and, if necessary, inserts new elements in their place, returning the deleted elements.
   * @param start The zero-based location in the array from which to start removing elements.
   * @param deleteCount The number of elements to remove.
   * @param items Elements to insert into the array in place of the deleted elements.
   * @returns An array containing the elements that were deleted.
   */
  toSpliced<F>(start: number, deleteCount: number, ...items: F[]): (F | T)[];

  /**
   * Copies and replaces the element at the given index with the provided value.
   * @param index The zero-based location in the array for which to replace an element.
   * @param value Element to insert into the array in place of the replaced element.
   */
  with<F>(index: number, value: F): (F | T)[];
 }

interface ReadonlyArray<T> {
  /** Copies and reverses the elements in an array.*/
  toReversed(): T[];

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if the first argument is less than the second argument, zero if they're equal, and a positive
   * value otherwise. If omitted, the elements are sorted in ascending, ASCII character order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: T, b: T) => number): T[];

  /**
   * Copies and elements from an array and, if necessary, inserts new elements in their place, returning the deleted elements.
   * @param start The zero-based location in the array from which to start removing elements.
   * @param deleteCount The number of elements to remove.
   * @returns An array containing the elements that were deleted.
   */
  toSpliced(start: number, deleteCount?: number): T[];
  /**
   * Copies and removes elements from an array and, if necessary, inserts new elements in their place, returning the deleted elements.
   * @param start The zero-based location in the array from which to start removing elements.
   * @param deleteCount The number of elements to remove.
   * @param items Elements to insert into the array in place of the deleted elements.
   * @returns An array containing the elements that were deleted.
   */
  toSpliced<F>(start: number, deleteCount: number, ...items: F[]): (F | T)[];

  /**
   * Copies and replaces the element at the given index with the provided value.
   * @param index The zero-based location in the array for which to replace an element.
   * @param value Element to insert into the array in place of the replaced element.
   */
  with<F>(index: number, value: F): (F | T)[];
}

interface Int8Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Int8Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Int8Array;

  with(index: number, value: number): Int8Array;
}

interface Uint8Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Uint8Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Uint8Array;

  with(index: number, value: number): Uint8Array;
}

interface Uint8ClampedArray {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Uint8ClampedArray;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Uint8ClampedArray;

  with(index: number, value: number): Uint8ClampedArray;
}


interface Int16Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Int16Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Int16Array;

  with(index: number, value: number): Int16Array;
}

interface Uint16Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Uint16Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Uint16Array;

  with(index: number, value: number): Uint16Array;
}

interface Int32Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Int32Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Int32Array;

  with(index: number, value: number): Int32Array;
}

interface Uint32Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Uint32Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Uint32Array;

  with(index: number, value: number): Uint32Array;
}

interface Float32Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Float32Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Float32Array;

  with(index: number, value: number): Float32Array;
}

interface Float64Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): Float64Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: number, b: number) => number): Float64Array;

  with(index: number, value: number): Float64Array;
}

interface BigInt64Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): BigInt64Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: bigint, b: bigint) => number | bigint): BigInt64Array;

  with(index: number, value: number): BigInt64Array;
}

interface BigUint64Array {
  /** Copies and reverses the elements in an array.*/
  toReversed(): BigUint64Array;

  /**
   * Copies and sorts an array.
   * @param compareFn Function used to determine the order of the elements. It is expected to return
   * a negative value if first argument is less than second argument, zero if they're equal and a positive
   * value otherwise. If omitted, the elements are sorted in ascending order.
   * ```ts
   * [11,2,22,1].sort((a, b) => a - b)
   * ```
   */
  toSorted(compareFn?: (a: bigint, b: bigint) => number | bigint): BigUint64Array;

  with(index: number, value: number): BigUint64Array;
}

// NOTE(bartlomieju): taken from https://github.com/microsoft/TypeScript/issues/50803#issuecomment-1249030430
// while we wait for these types to officially ship
interface ArrayConstructor {
  fromAsync<T>(
      iterableOrArrayLike: AsyncIterable<T> | Iterable<T | Promise<T>> | ArrayLike<T | Promise<T>>,
  ): Promise<T[]>;
  
  fromAsync<T, U>(
      iterableOrArrayLike: AsyncIterable<T> | Iterable<T> | ArrayLike<T>, 
      mapFn: (value: Awaited<T>) => U, 
      thisArg?: any,
  ): Promise<Awaited<U>[]>;
}
