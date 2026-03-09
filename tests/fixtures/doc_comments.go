package main

// Greet returns a greeting string.
// It takes a name parameter.
func Greet(name string) string {
	return "Hello, " + name
}

// unexported has a comment too.
func unexported() {}

func noComment() {}
