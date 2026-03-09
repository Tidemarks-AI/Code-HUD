/// <summary>
/// A documented class.
/// </summary>
public class DocComments {
    /// <summary>
    /// XML doc on a method.
    /// </summary>
    /// <param name="name">The name.</param>
    public string Greet(string name) {
        return "Hello, " + name;
    }

    // Regular comment.
    public void NoDoc() {}
}
