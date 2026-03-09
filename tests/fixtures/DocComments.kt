/**
 * A documented class.
 */
class DocComments {
    /**
     * KDoc on a method.
     * @param name the name
     * @return greeting
     */
    fun greet(name: String): String {
        return "Hello, $name"
    }

    // Not a KDoc comment.
    fun noDoc() {}
}
