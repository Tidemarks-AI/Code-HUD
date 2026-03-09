/**
 * A documented function.
 * @param {string} name
 */
function greet(name) {
    return "Hello, " + name;
}

// Not a JSDoc comment.
function noJsDoc() {}

/**
 * Documented class.
 */
class Widget {
    /** Constructor doc. */
    constructor(id) {
        this.id = id;
    }
}
