package main

import (
	"fmt"
	"strings"
)

// User represents a user in the system.
type User struct {
	Name  string
	Email string
	age   int
}

// Stringer is implemented by types that can convert to string.
type Stringer interface {
	String() string
}

// NewUser creates a new User.
func NewUser(name, email string) *User {
	return &User{Name: name, Email: email}
}

func (u *User) String() string {
	return fmt.Sprintf("%s <%s>", u.Name, u.Email)
}

func (u *User) setAge(a int) {
	u.age = a
}

// MaxRetries is the maximum number of retries.
const MaxRetries = 3

var defaultTimeout = 30

type StringAlias = string

func helperFunc() {
	fmt.Println(strings.TrimSpace("hello"))
}
