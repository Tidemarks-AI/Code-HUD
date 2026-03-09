/**
 * A documented interface.
 */
export interface Documented {
    name: string;
}

// Regular comment, not JSDoc.
export function notJsDoc(): void {}

/**
 * JSDoc on a function.
 * @param x - a number
 */
export function jsDocFunc(x: number): number {
    return x;
}
