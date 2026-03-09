/**
 * A documented function.
 * @param name the name
 * @return greeting string
 */
std::string greet(const std::string& name) {
    return "Hello, " + name;
}

// Regular comment, not Doxygen.
void noDoc() {}

/**
 * Documented class.
 */
class Widget {
public:
    /** Constructor doc. */
    Widget(int id) : id_(id) {}
private:
    int id_;
};
