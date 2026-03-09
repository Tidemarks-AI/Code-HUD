/**
 * A documented class.
 */
public class DocComments {
    /**
     * JavaDoc on a method.
     * @param name the name
     * @return greeting string
     */
    public String greet(String name) {
        return "Hello, " + name;
    }

    // Not a javadoc comment.
    public void noDoc() {}
}
